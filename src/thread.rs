use {
    super::{app::NodeExprs, expr::Expr},
    crossbeam_channel::{unbounded, Receiver, Sender},
    std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    },
};

#[cfg(not(target_arch = "wasm32"))]
use std::{
    iter::repeat_with,
    num::NonZeroUsize,
    thread::{available_parallelism, spawn, JoinHandle},
};

type NodeExprsCache = HashMap<usize, (usize, Arc<Expr>)>;

#[derive(Clone, Copy)]
pub struct ImageInfo {
    pub coord: u8,
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

pub struct Threads {
    #[cfg(target_arch = "wasm32")]
    worker: Box<dyn Fn()>,

    #[cfg(not(target_arch = "wasm32"))]
    workers: Vec<JoinHandle<()>>,

    rx: Receiver<(usize, usize, u8, [u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE])>,
    tx: Sender<Option<(usize, usize, ImageInfo)>>,
}

impl Threads {
    /// The number of image coordinates along any one side of an image.
    ///
    /// We use 16 because an image is chunked into 256 sub-images (16 x 16). This coordinate allows
    /// threads to send and receive the location of a sub-image easily.
    pub const IMAGE_COORDS: u8 = 16;

    /// The number of pixels along any one side of a sub-image.
    pub const IMAGE_SIZE: usize = 8;

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

    pub fn coord_to_row_col(coord: u8) -> [usize; 2] {
        let row = (coord / Self::IMAGE_COORDS) as usize * Self::IMAGE_SIZE;
        let col = (coord % Self::IMAGE_COORDS) as usize * Self::IMAGE_SIZE;

        [row, col]
    }

    fn process_request(
        node_exprs: &Arc<RwLock<NodeExprsCache>>,
        node_idx: usize,
        version: usize,
        image_info: ImageInfo,
        tx: &Sender<(usize, usize, u8, [u8; 64])>,
    ) -> bool {
        let ImageInfo { coord, scale, x, y } = image_info;

        // Double-check that the expression is still the current version (it may have been
        // updated by the time we receive this request)
        if let Some(expr) = node_exprs
            .read()
            .unwrap()
            .get(&node_idx)
            .filter(|(current_version, _)| *current_version == version)
            .map(|(_, expr)| Arc::clone(expr))
        {
            let [row, col] = Self::coord_to_row_col(coord);
            let step = 1.0 / (Self::IMAGE_SIZE * 16) as f64;
            let half_step = step / 2.0;
            let mut image = [0u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE];

            for image_y in 0..Self::IMAGE_SIZE {
                let eval_y = ((row + image_y) as f64 * step + half_step + x) * scale;
                for image_x in 0..Self::IMAGE_SIZE {
                    let eval_x = ((col + image_x) as f64 * step + half_step + y) * scale;
                    let sample = (expr.noise().get([eval_x, eval_y, 0.0]) + 1.0) / 2.0;
                    image[image_x * Self::IMAGE_SIZE + image_y] = (sample * 255.0) as u8;
                }
            }

            tx.send((node_idx, version, coord, image)).unwrap();

            true
        } else {
            false
        }
    }

    pub fn send(&self, node: usize, version: usize, image_info: ImageInfo) {
        self.tx.send(Some((node, version, image_info))).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn thread_worker(
        node_exprs: NodeExprs,
        rx: Receiver<Option<(usize, usize, ImageInfo)>>,
        tx: Sender<(usize, usize, u8, [u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE])>,
    ) {
        // Receive the next versioned node request from the main thread
        while let Some((node_idx, version, image_info)) = rx.recv().unwrap() {
            Self::process_request(&node_exprs, node_idx, version, image_info, &tx);
        }
    }

    pub fn try_recv_iter(
        &self,
    ) -> impl Iterator<Item = (usize, usize, u8, [u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE])> + '_
    {
        self.rx.try_iter()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update(&self) {
        self.worker.as_ref()();
    }

    #[cfg(target_arch = "wasm32")]
    fn web_worker(
        node_exprs: &NodeExprs,
        rx: &Receiver<Option<(usize, usize, ImageInfo)>>,
        tx: &Sender<(usize, usize, u8, [u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE])>,
    ) {
        // On web we only process a small number of requests, always checking to only count
        // requests which are actually processed (and not stale ones)
        let mut processed = 0;

        // Receive the next versioned node request
        while let Some((node_idx, version, image_info)) = rx.try_recv().ok().flatten() {
            if Self::process_request(&node_exprs, node_idx, version, image_info, &tx) {
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
