use {
    super::{app::NodeExprs, node::ImagePlane},
    crossbeam_channel::{unbounded, Receiver, Sender},
    egui_snarl::NodeId,
    noise_expr::Expr,
    std::{
        collections::HashMap,
        ops::Range,
        sync::{Arc, RwLock},
    },
};

#[cfg(not(target_arch = "wasm32"))]
use std::{
    iter::repeat_with,
    num::NonZeroUsize,
    thread::{available_parallelism, spawn, JoinHandle},
};

type NodeExprsCache = HashMap<NodeId, (usize, Arc<Expr>)>;

#[derive(Clone)]
pub struct ImageInfo {
    pub coord: u8,
    pub transform: ImagePlane,
    pub value: Range<f64>,
    pub x: Range<f64>,
    pub y: Range<f64>,
}

pub struct Threads {
    #[cfg(target_arch = "wasm32")]
    worker: Box<dyn Fn()>,

    #[cfg(not(target_arch = "wasm32"))]
    workers: Vec<JoinHandle<()>>,

    rx: Receiver<(NodeId, usize, u8, Box<[u8]>)>,
    tx: Sender<Option<(NodeId, usize, ImageInfo)>>,
}

impl Threads {
    /// The number of image coordinates along any one side of an image.
    ///
    /// We use 16 because an image is chunked into 256 sub-images (16 x 16). This coordinate allows
    /// threads to send and receive the location of a sub-image easily.
    pub const IMAGE_COORDS: u8 = 16;

    /// The number of pixels along any one side of a sub-image.
    pub const SUB_IMAGE_SIZE: u32 = 8;

    #[cfg(target_arch = "wasm32")]
    const REQUESTS_PER_FRAME: usize = 64;

    pub fn new(node_exprs: &NodeExprs) -> Self {
        let (tx, thread_rx) = unbounded();
        let (thread_tx, rx) = unbounded();

        #[cfg(target_arch = "wasm32")]
        let worker = {
            let node_exprs = Arc::clone(node_exprs);
            let (tx, rx) = (thread_tx.clone(), thread_rx.clone());

            Box::new(move || {
                Self::web_worker(&node_exprs, &rx, &tx);
            })
        };

        #[cfg(not(target_arch = "wasm32"))]
        let workers = repeat_with(|| {
            let node_exprs = Arc::clone(node_exprs);
            let (tx, rx) = (thread_tx.clone(), thread_rx.clone());
            spawn(|| Self::thread_worker(node_exprs, rx, tx))
        })
        .take(
            available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or_default()
                .max(1),
        )
        .collect();

        Self {
            #[cfg(target_arch = "wasm32")]
            worker,

            #[cfg(not(target_arch = "wasm32"))]
            workers,

            rx,
            tx,
        }
    }

    pub fn coord_to_col_row(coord: u8) -> [u32; 2] {
        let row = (coord / Self::IMAGE_COORDS) as u32 * Self::SUB_IMAGE_SIZE;
        let col = (coord % Self::IMAGE_COORDS) as u32 * Self::SUB_IMAGE_SIZE;

        [col, row]
    }

    fn process_request(
        node_exprs: &Arc<RwLock<NodeExprsCache>>,
        node_id: NodeId,
        version: usize,
        image_info: ImageInfo,
        tx: &Sender<(NodeId, usize, u8, Box<[u8]>)>,
    ) -> bool {
        let ImageInfo {
            coord,
            transform,
            value,
            x,
            y,
        } = image_info;

        // Double-check that the expression is still the current version (it may have been
        // updated by the time we receive this request)
        if let Some(expr) = node_exprs
            .read()
            .unwrap()
            .get(&node_id)
            .filter(|(current_version, _)| *current_version == version)
            .map(|(_, expr)| Arc::clone(expr))
        {
            const INV_IMAGE_SIZE: f64 =
                1.0 / (Threads::SUB_IMAGE_SIZE * Threads::IMAGE_COORDS as u32) as f64;

            let mut image =
                vec![0u8; (Self::SUB_IMAGE_SIZE * Self::SUB_IMAGE_SIZE) as _].into_boxed_slice();
            let [image_col, image_row] = Self::coord_to_col_row(coord);

            let transform_point = match transform {
                ImagePlane::XY => |x, y| [x, y, 0.0],
                ImagePlane::XZ => |x, y| [x, 0.0, y],
                ImagePlane::ZY => |x, y| [0.0, y, x],
            };

            let noise = expr.noise();
            let range_value = 1.0 / (value.end - value.start);
            let range_x = INV_IMAGE_SIZE * (x.end - x.start);
            let range_y = INV_IMAGE_SIZE * (y.end - y.start);

            for sub_image_y in 0..Self::SUB_IMAGE_SIZE {
                let eval_y = (image_row + sub_image_y) as f64 * range_y + y.start;
                for sub_image_x in 0..Self::SUB_IMAGE_SIZE {
                    const MAX: f64 = u8::MAX as f64;
                    let eval_x = (image_col + sub_image_x) as f64 * range_x + x.start;
                    let eval_pt = transform_point(eval_x, eval_y);
                    let value = (noise.get(eval_pt) - value.start) * range_value;
                    let idx = Self::sub_image_coord_to_index(sub_image_x, sub_image_y);
                    image[idx] = (value * MAX).clamp(0.0, MAX) as _;
                }
            }

            tx.send((node_id, version, coord, image)).unwrap();

            true
        } else {
            false
        }
    }

    pub fn send(&self, node: NodeId, version: usize, image_info: ImageInfo) {
        self.tx.send(Some((node, version, image_info))).unwrap();
    }

    fn sub_image_coord_to_index(x: u32, y: u32) -> usize {
        (y * Self::SUB_IMAGE_SIZE + x) as _
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn thread_worker(
        node_exprs: NodeExprs,
        rx: Receiver<Option<(NodeId, usize, ImageInfo)>>,
        tx: Sender<(NodeId, usize, u8, Box<[u8]>)>,
    ) {
        // Receive the next versioned node request from the main thread
        while let Some((node_id, version, image_info)) = rx.recv().unwrap() {
            Self::process_request(&node_exprs, node_id, version, image_info, &tx);
        }
    }

    pub fn try_recv_iter(&self) -> impl Iterator<Item = (NodeId, usize, u8, Box<[u8]>)> + '_ {
        self.rx.try_iter()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update(&self) {
        self.worker.as_ref()();
    }

    #[cfg(target_arch = "wasm32")]
    fn web_worker(
        node_exprs: &NodeExprs,
        rx: &Receiver<Option<(NodeId, usize, ImageInfo)>>,
        tx: &Sender<(NodeId, usize, u8, Box<[u8]>)>,
    ) {
        // On web we only process a small number of requests, always checking to only count
        // requests which are actually processed (and not stale ones)
        let mut processed = 0;

        // Receive the next versioned node request
        while let Some((node_id, version, image_info)) = rx.try_recv().ok().flatten() {
            if Self::process_request(&node_exprs, node_id, version, image_info, &tx) {
                processed += 1;

                if processed == Self::REQUESTS_PER_FRAME {
                    return;
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Threads {
    fn drop(&mut self) {
        for _ in 0..self.workers.len() {
            self.tx.send(None).unwrap();
        }

        for worker in self.workers.drain(..) {
            worker.join().unwrap();
        }
    }
}
