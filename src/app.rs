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
        Id, Layout,
    },
    egui_snarl::{ui::SnarlStyle, OutPinId, Snarl},
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

pub type NodeExprs = Arc<RwLock<HashMap<usize, (usize, Arc<Expr>)>>>;

pub struct App {
    node_exprs: NodeExprs,

    #[cfg(not(target_arch = "wasm32"))]
    path: Option<PathBuf>,

    snarl: Snarl<NoiseNode>,
    threads: Threads,
    removed_node_indices: HashSet<usize>,
    updated_node_indices: HashSet<usize>,
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
        let removed_node_indices = Default::default();
        let updated_node_indices = Self::all_image_node_indices(&snarl).collect();

        Self {
            node_exprs,

            #[cfg(not(target_arch = "wasm32"))]
            path: None,

            snarl,
            threads,
            removed_node_indices,
            updated_node_indices,
            version: 0,
        }
    }

    fn all_image_node_indices(snarl: &Snarl<NoiseNode>) -> impl Iterator<Item = usize> + '_ {
        snarl
            .node_indices()
            .filter_map(|(node_idx, node)| node.has_image().then_some(node_idx))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn file_dialog() -> FileDialog {
        FileDialog::new().add_filter("Noise Project", &[Self::EXTENSION])
    }

    fn has_changes(&self) -> bool {
        !self.removed_node_indices.is_empty() || !self.updated_node_indices.is_empty()
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

        for node_idx in self.removed_node_indices.drain() {
            node_exprs.remove(&node_idx);

            // Just in case (never happens!)
            self.updated_node_indices.remove(&node_idx);
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
            static NODE_INDICES: RefCell<Option<HashSet<usize>>> = RefCell::new(Some(Default::default()));
        }

        // HACK: snarl::get_node_mut doesn't return an option and will panic on missing node indices
        let mut node_indices = NODE_INDICES.take().unwrap();
        for (node_idx, _) in self.snarl.node_indices() {
            node_indices.insert(node_idx);
        }

        for (node_idx, image_version, coord, image) in self.threads.try_recv_iter() {
            // We have to check to make sure snarl *still* contains this index because it may have
            // been removed by the time the thread has responded to the image request
            if !node_indices.contains(&node_idx) {
                continue;
            }

            if let Some(Image {
                texture: Some(texture),
                version,
                ..
            }) = self.snarl.get_node_mut(node_idx).image_mut()
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

        node_indices.clear();
        NODE_INDICES.set(Some(node_indices));
    }

    fn update_nodes(&mut self, ctx: &Context) {
        thread_local! {
            static CHILD_NODE_INDICES: RefCell<Option<HashSet<usize>>> = RefCell::new(Some(Default::default()));
            static TEMP_NODE_INDICES: RefCell<Option<Vec<usize>>> = RefCell::new(Some(Default::default()));
        }

        let mut child_node_indices = CHILD_NODE_INDICES.take().unwrap();
        let mut temp_node_indices = TEMP_NODE_INDICES.take().unwrap();

        // Before we process the user-updated nodes, we must propagate updates to child nodes
        for node_idx in self.updated_node_indices.iter().copied() {
            temp_node_indices.push(node_idx);
            while let Some(node_idx) = temp_node_indices.pop() {
                for node_idx in self
                    .snarl
                    .out_pin(OutPinId {
                        node: node_idx,
                        output: 0,
                    })
                    .remotes
                    .iter()
                    .map(|remote| remote.node)
                {
                    child_node_indices.insert(node_idx);
                    temp_node_indices.push(node_idx);
                }
            }
        }

        self.updated_node_indices.extend(child_node_indices.drain());
        CHILD_NODE_INDICES.set(Some(child_node_indices));
        TEMP_NODE_INDICES.set(Some(temp_node_indices));

        // First we update the version of all updated images
        self.version = self.version.wrapping_add(1);
        for node_idx in self.updated_node_indices.iter().copied() {
            let node = self.snarl.get_node_mut(node_idx);
            if let Some(image) = node.image_mut() {
                // Ensure all image nodes contain a valid texture
                if image.texture.is_none() {
                    debug!("Creating image for #{node_idx}");

                    image.texture = Some(ctx.load_texture(
                        format!("image{node_idx}"),
                        ColorImage::new(Self::IMAGE_SIZE, Color32::TRANSPARENT),
                        Default::default(),
                    ));
                }

                image.version = self.version;
            }
        }

        type Request = (usize, usize, ImageInfo);

        thread_local! {
            static REQUESTS: RefCell<Option<Vec<Request>>> = RefCell::new(Some(Default::default()));
        }

        let mut requests = REQUESTS.take().unwrap();

        // Next we update the expressions of all updated images and request new images
        for node_idx in self.updated_node_indices.drain() {
            let node = self.snarl.get_node(node_idx);
            if let Some(image) = node.image() {
                debug!("Updating image for #{node_idx}");

                self.node_exprs.write().unwrap().insert(
                    node_idx,
                    (image.version, Arc::new(node.expr(node_idx, &self.snarl))),
                );

                // We request coordinate chunks from the threads using pre-shuffled data so that
                // all the responses come back in a static-like pattern and not row by row
                for coord in shuffled_u8(image.version).iter().copied() {
                    requests.push((
                        node_idx,
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
                let (node_idx, image_version, image_info) =
                    requests[image_idx * Self::IMAGE_COUNT + request_idx];
                self.threads.send(node_idx, image_version, image_info);
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
                            self.updated_node_indices =
                                Self::all_image_node_indices(&self.snarl).collect();
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

                widgets::global_dark_light_mode_buttons(ui);
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            self.snarl.show(
                &mut Viewer {
                    removed_node_indices: &mut self.removed_node_indices,
                    updated_node_indices: &mut self.updated_node_indices,
                },
                &SnarlStyle {
                    collapsible: true,
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
