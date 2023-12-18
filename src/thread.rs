use {
    super::app::NodeExprs,
    crossbeam_channel::{unbounded, Receiver, Sender},
    std::{
        iter::repeat_with,
        num::NonZeroUsize,
        sync::Arc,
        thread::{available_parallelism, spawn, JoinHandle},
    },
};

#[derive(Clone, Copy)]
pub struct ImageInfo {
    pub coord: u8,
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

pub struct Threads {
    join_handles: Vec<JoinHandle<()>>,
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

    pub fn new(node_exprs: &NodeExprs) -> Self {
        let (tx, thread_rx) = unbounded();
        let (thread_tx, rx) = unbounded();
        let join_handles = repeat_with(|| {
            let node_exprs = Arc::clone(node_exprs);
            let (tx, rx) = (thread_tx.clone(), thread_rx.clone());
            spawn(|| Self::thread(node_exprs, rx, tx))
        })
        .take(
            available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or_default()
                .max(1),
        )
        .collect();

        Self {
            join_handles,
            rx,
            tx,
        }
    }

    pub fn coord_to_row_col(coord: u8) -> [usize; 2] {
        let row = (coord / Self::IMAGE_COORDS) as usize * Self::IMAGE_SIZE;
        let col = (coord % Self::IMAGE_COORDS) as usize * Self::IMAGE_SIZE;

        [row, col]
    }

    pub fn send(&self, node: usize, version: usize, image_info: ImageInfo) {
        self.tx.send(Some((node, version, image_info))).unwrap();
    }

    fn thread(
        node_exprs: NodeExprs,
        rx: Receiver<Option<(usize, usize, ImageInfo)>>,
        tx: Sender<(usize, usize, u8, [u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE])>,
    ) {
        // Receive the next versioned node request from the main thread
        while let Some((node_idx, version, ImageInfo { coord, scale, x, y })) = rx.recv().unwrap() {
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
            }
        }
    }

    pub fn try_recv_iter(
        &self,
    ) -> impl Iterator<Item = (usize, usize, u8, [u8; Self::IMAGE_SIZE * Self::IMAGE_SIZE])> + '_
    {
        self.rx.try_iter()
    }
}

impl Drop for Threads {
    fn drop(&mut self) {
        for _ in 0..self.join_handles.len() {
            self.tx.send(None).unwrap();
        }

        for join_handle in self.join_handles.drain(..) {
            join_handle.join().unwrap();
        }
    }
}
