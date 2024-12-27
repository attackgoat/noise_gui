use {
    super::{
        expr::Expr,
        node::{Image, NoiseNode},
        rand::shuffled_u8,
        thread::{ImageInfo, Threads},
        view::Viewer,
    },
    eframe::{get_value, set_value, CreationContext, Frame, Storage, APP_KEY},
    egui::{
        github_link_file, warn_if_debug_build, Align, CentralPanel, Color32, ColorImage, Context,
        Id, Layout, Vec2,
    },
    egui_snarl::{
        ui::{BackgroundPattern, Grid, SnarlStyle},
        NodeId, OutPinId, Snarl,
    },
    log::debug,
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        sync::{Arc, RwLock},
    },
};

#[cfg(not(target_arch = "wasm32"))]
use {
    egui::{menu, widgets, TopBottomPanel, ViewportCommand},
    log::warn,
    rfd::FileDialog,
    ron::{
        de::from_reader,
        ser::{to_writer_pretty, PrettyConfig},
    },
    serde::Serialize,
    std::{
        fs::OpenOptions,
        path::{Path, PathBuf},
    },
};

pub type NodeExprs = Arc<RwLock<HashMap<NodeId, (usize, Arc<Expr>)>>>;

pub struct App {
    node_exprs: NodeExprs,

    #[cfg(not(target_arch = "wasm32"))]
    path: Option<PathBuf>,

    snarl: Snarl<NoiseNode>,
    threads: Threads,
    removed_node_ids: HashSet<NodeId>,
    updated_node_ids: HashSet<NodeId>,
    version: usize,
}

impl App {
    #[cfg(not(target_arch = "wasm32"))]
    pub const EXTENSION: &'static str = "ron";

    const IMAGE_COUNT: usize = Threads::IMAGE_COORDS as usize * Threads::IMAGE_COORDS as usize;
    const IMAGE_SIZE: [usize; 2] = [
        Threads::IMAGE_SIZE * Threads::IMAGE_COORDS as usize,
        Threads::IMAGE_SIZE * Threads::IMAGE_COORDS as usize,
    ];

    pub fn new(#[allow(unused_variables)] cc: &CreationContext<'_>) -> Self {
        let snarl: Snarl<NoiseNode> = if let Some(storage) = cc.storage {
            get_value(storage, APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let node_exprs = Default::default();
        let threads = Threads::new(&node_exprs);
        let removed_node_ids = Default::default();
        let updated_node_ids = Self::all_image_node_ids(&snarl).collect();

        Self {
            node_exprs,

            #[cfg(not(target_arch = "wasm32"))]
            path: None,

            snarl,
            threads,
            removed_node_ids,
            updated_node_ids,
            version: 0,
        }
    }

    fn all_image_node_ids(snarl: &Snarl<NoiseNode>) -> impl Iterator<Item = NodeId> + '_ {
        snarl
            .node_ids()
            .filter_map(|(node_id, node)| node.has_image().then_some(node_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn file_dialog() -> FileDialog {
        FileDialog::new().add_filter("Noise Project", &[Self::EXTENSION])
    }

    fn has_changes(&self) -> bool {
        !self.removed_node_ids.is_empty() || !self.updated_node_ids.is_empty()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open(path: impl AsRef<Path>) -> anyhow::Result<Snarl<NoiseNode>> {
        Ok(
            from_reader(OpenOptions::new().read(true).open(path).map_err(|err| {
                warn!("Unable to open file");
                err
            })?)
            .map_err(|err: ron::error::SpannedError| {
                warn!("Unable to read file");
                err
            })?,
        )
    }

    fn remove_nodes(&mut self) {
        let mut node_exprs = self.node_exprs.write().unwrap();

        for node_id in self.removed_node_ids.drain() {
            node_exprs.remove(&node_id);

            // Just in case (never happens!)
            self.updated_node_ids.remove(&node_id);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as<T>(path: impl AsRef<Path>, value: &T) -> anyhow::Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut path = path.as_ref().to_path_buf();

        if path.extension().is_none() {
            path.set_extension(Self::EXTENSION);
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|err| {
                warn!("Unable to create file");
                err
            })?;
        to_writer_pretty(file, value, PrettyConfig::default()).map_err(|err| {
            warn!("Unable to write file");
            err
        })?;

        Ok(())
    }

    fn update_images(&mut self) {
        thread_local! {
            static NODE_IDS: RefCell<Option<HashSet<NodeId>>> = RefCell::new(Some(Default::default()));
        }

        // HACK: snarl::get_node_mut doesn't return an option and will panic on missing node indices
        let mut node_ids = NODE_IDS.take().unwrap();
        for (node_id, _) in self.snarl.node_ids() {
            node_ids.insert(node_id);
        }

        for (node_id, image_version, coord, image) in self.threads.try_recv_iter() {
            // We have to check to make sure snarl *still* contains this index because it may have
            // been removed by the time the thread has responded to the image request
            if !node_ids.contains(&node_id) {
                continue;
            }

            if let Some(Image {
                texture: Some(texture),
                version,
                ..
            }) = self
                .snarl
                .get_node_mut(node_id)
                .and_then(NoiseNode::image_mut)
            {
                // We have to check to make sure the current image version is the same one the
                // thread has responded with - if not a new request will be received later
                if *version != image_version {
                    continue;
                }

                texture.set_partial(
                    Threads::coord_to_row_col(coord),
                    ColorImage::from_gray([Threads::IMAGE_SIZE, Threads::IMAGE_SIZE], &image),
                    Default::default(),
                );
            }
        }

        node_ids.clear();
        NODE_IDS.set(Some(node_ids));
    }

    fn update_nodes(&mut self, ctx: &Context) {
        thread_local! {
            static CHILD_NODE_IDS: RefCell<Option<HashSet<NodeId>>> = RefCell::new(Some(Default::default()));
            static TEMP_NODE_IDS: RefCell<Option<Vec<NodeId>>> = RefCell::new(Some(Default::default()));
        }

        let mut child_node_ids = CHILD_NODE_IDS.take().unwrap();
        let mut temp_node_ids = TEMP_NODE_IDS.take().unwrap();

        // Before we process the user-updated nodes, we must propagate updates to child nodes
        for node_id in self.updated_node_ids.iter().copied() {
            temp_node_ids.push(node_id);
            while let Some(node_id) = temp_node_ids.pop() {
                for node_id in self
                    .snarl
                    .out_pin(OutPinId {
                        node: node_id,
                        output: 0,
                    })
                    .remotes
                    .iter()
                    .map(|remote| remote.node)
                {
                    child_node_ids.insert(node_id);
                    temp_node_ids.push(node_id);
                }
            }
        }

        self.updated_node_ids.extend(child_node_ids.drain());
        CHILD_NODE_IDS.set(Some(child_node_ids));
        TEMP_NODE_IDS.set(Some(temp_node_ids));

        // First we update the version of all updated images
        self.version = self.version.wrapping_add(1);
        for node_id in self.updated_node_ids.iter().copied() {
            if let Some(image) = self
                .snarl
                .get_node_mut(node_id)
                .and_then(NoiseNode::image_mut)
            {
                // Ensure all image nodes contain a valid texture
                if image.texture.is_none() {
                    debug!("Creating image for #{node_id:?}");

                    image.texture = Some(ctx.load_texture(
                        format!("image{node_id:?}"),
                        ColorImage::new(Self::IMAGE_SIZE, Color32::TRANSPARENT),
                        Default::default(),
                    ));
                }

                image.version = self.version;
            }
        }

        type Request = (NodeId, usize, ImageInfo);

        thread_local! {
            static REQUESTS: RefCell<Option<Vec<Request>>> = RefCell::new(Some(Default::default()));
        }

        let mut requests = REQUESTS.take().unwrap();

        // Next we update the expressions of all updated images and request new images
        for node_id in self.updated_node_ids.drain() {
            let node = self.snarl.get_node(node_id).unwrap();
            if let Some(image) = node.image() {
                debug!("Updating image for #{node_id:?}");

                self.node_exprs.write().unwrap().insert(
                    node_id,
                    (image.version, Arc::new(node.expr(node_id, &self.snarl))),
                );

                // We request coordinate chunks from the threads using pre-shuffled data so that
                // all the responses come back in a static-like pattern and not row by row
                for coord in shuffled_u8(image.version).iter().copied() {
                    requests.push((
                        node_id,
                        image.version,
                        ImageInfo {
                            coord,
                            scale: image.scale,
                            x: image.x,
                            y: image.y,
                        },
                    ));
                }
            }
        }

        // All requests (which can be for multiple images) are sent in interleaved order so that
        // frequent requests don't always hit one image and cause the others to appear paused
        let image_count = requests.len() / Self::IMAGE_COUNT;
        for request_idx in 0..Self::IMAGE_COUNT {
            for image_idx in 0..image_count {
                let (node_id, image_version, image_info) =
                    requests[image_idx * Self::IMAGE_COUNT + request_idx];
                self.threads.send(node_id, image_version, image_info);
            }
        }

        requests.clear();
        REQUESTS.set(Some(requests));
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn Storage) {
        set_value(storage, APP_KEY, &self.snarl);
    }

    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        #[cfg(target_arch = "wasm32")]
        self.threads.update();

        self.update_images();

        #[cfg(not(target_arch = "wasm32"))]
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.path = None;
                        self.snarl = Snarl::new();

                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Open File...").clicked() {
                        if let Some(path) = Self::file_dialog().pick_file() {
                            self.snarl = Self::open(&path).unwrap_or_default();
                            self.path = Some(path);
                            self.updated_node_ids = Self::all_image_node_ids(&self.snarl).collect();
                        }

                        ui.close_menu();
                    }

                    if let Some(path) = &self.path {
                        if ui.button("Save").clicked() {
                            Self::save_as(path, &self.snarl).unwrap_or_default();

                            ui.close_menu();
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.add_space(2.0);
                            ui.label("Save");
                        });
                    }

                    if ui.button("Save As...").clicked() {
                        if let Some(path) = Self::file_dialog().save_file() {
                            Self::save_as(&path, &self.snarl).unwrap_or_default();
                            self.path = Some(path);
                        }

                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                widgets::global_theme_preference_switch(ui);
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            self.snarl.show(
                &mut Viewer {
                    removed_node_ids: &mut self.removed_node_ids,
                    updated_node_ids: &mut self.updated_node_ids,
                },
                &SnarlStyle {
                    bg_pattern: Some(BackgroundPattern::Grid(Grid::new(
                        Vec2::new(20.0, 20.0),
                        0.0,
                    ))),
                    collapsible: Some(true),
                    ..Default::default()
                },
                Id::new("snarl"),
                ui,
            );
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                ui.add(github_link_file!(
                    "https://github.com/attackgoat/noise_gui/blob/master/",
                    "Source code"
                ));

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("Powered by ");
                    ui.hyperlink_to("egui-snarl", "https://github.com/zakarumych/egui-snarl");
                });

                warn_if_debug_build(ui);
            });
        });

        if self.has_changes() {
            self.remove_nodes();
            self.update_nodes(ctx);
        }
    }
}
