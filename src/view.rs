use {
    super::node::{
        AbsNode, CombinerNode, ConstantNode, FractalNode, NodeInput, NoiseNode, PerlinNode,
        RigidFractalNode, Source,
    },
    egui::{Color32, RichText},
    egui_snarl::{
        ui::{InPin, OutPin, PinInfo, SnarlViewer},
        Snarl,
    },
    log::debug,
    std::collections::HashSet,
};

pub struct Viewer<'a> {
    pub removed_node_indices: &'a mut HashSet<usize>,
    pub updated_node_indices: &'a mut HashSet<usize>,
}

impl<'a> Viewer<'a> {
    fn source_combo_box(&mut self, ui: &mut egui::Ui, source: &mut Source, node_idx: usize) {
        egui::ComboBox::from_id_source(0)
            .selected_text(format!("{source:?}"))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for value in [
                    Source::OpenSimplex,
                    Source::Perlin,
                    Source::PerlinSurflet,
                    Source::Simplex,
                    Source::SuperSimplex,
                    Source::Value,
                    Source::Worley,
                ] {
                    if ui
                        .selectable_value(source, value, format!("{value:?}"))
                        .changed()
                    {
                        self.updated_node_indices.insert(node_idx);
                    }
                }
            });
    }

    fn drag_value_f64(&mut self, ui: &mut egui::Ui, value: &mut f64, pin: &InPin) {
        if ui
            .add(
                egui::DragValue::new(value)
                    .min_decimals(2)
                    .max_decimals(2)
                    .speed(0.01),
            )
            .changed()
        {
            self.updated_node_indices.insert(pin.id.node);
        }
    }
}

impl<'a> SnarlViewer<NoiseNode> for Viewer<'a> {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NoiseNode>) {
        let from_node = snarl.get_node(from.id.node).clone();
        let to_node = snarl.get_node_mut(to.id.node);

        match (from_node, to_node) {
            (
                NoiseNode::Abs(_)
                | NoiseNode::Add(_)
                | NoiseNode::BasicMulti(_)
                | NoiseNode::Billow(_)
                | NoiseNode::F64(_)
                | NoiseNode::Fbm(_)
                | NoiseNode::HybridMulti(_)
                | NoiseNode::Min(_)
                | NoiseNode::Max(_)
                | NoiseNode::Multiply(_)
                | NoiseNode::Perlin(_)
                | NoiseNode::Power(_)
                | NoiseNode::RigidMulti(_),
                NoiseNode::Abs(_)
                | NoiseNode::Add(_)
                | NoiseNode::Min(_)
                | NoiseNode::Max(_)
                | NoiseNode::Multiply(_)
                | NoiseNode::Power(_),
            ) => {}
            (
                NoiseNode::U32(_),
                NoiseNode::BasicMulti(FractalNode { seed, .. })
                | NoiseNode::Billow(FractalNode { seed, .. })
                | NoiseNode::Fbm(FractalNode { seed, .. })
                | NoiseNode::Perlin(PerlinNode { seed, .. })
                | NoiseNode::HybridMulti(FractalNode { seed, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { seed, .. }),
            ) if to.id.input == 0 => {
                *seed = NodeInput::Node(from.id.node);
            }
            (
                NoiseNode::U32(_),
                NoiseNode::BasicMulti(FractalNode { octaves, .. })
                | NoiseNode::Billow(FractalNode { octaves, .. })
                | NoiseNode::Fbm(FractalNode { octaves, .. })
                | NoiseNode::HybridMulti(FractalNode { octaves, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { octaves, .. }),
            ) if to.id.input == 1 => {
                *octaves = NodeInput::Node(from.id.node);
            }
            (
                NoiseNode::F64(_),
                NoiseNode::BasicMulti(FractalNode { frequency, .. })
                | NoiseNode::Billow(FractalNode { frequency, .. })
                | NoiseNode::Fbm(FractalNode { frequency, .. })
                | NoiseNode::HybridMulti(FractalNode { frequency, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { frequency, .. }),
            ) if to.id.input == 2 => {
                *frequency = NodeInput::Node(from.id.node);
            }
            (
                NoiseNode::F64(_),
                NoiseNode::BasicMulti(FractalNode { lacunarity, .. })
                | NoiseNode::Billow(FractalNode { lacunarity, .. })
                | NoiseNode::Fbm(FractalNode { lacunarity, .. })
                | NoiseNode::HybridMulti(FractalNode { lacunarity, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { lacunarity, .. }),
            ) if to.id.input == 3 => {
                *lacunarity = NodeInput::Node(from.id.node);
            }
            (
                NoiseNode::F64(_),
                NoiseNode::BasicMulti(FractalNode { persistence, .. })
                | NoiseNode::Billow(FractalNode { persistence, .. })
                | NoiseNode::Fbm(FractalNode { persistence, .. })
                | NoiseNode::HybridMulti(FractalNode { persistence, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { persistence, .. }),
            ) if to.id.input == 4 => {
                *persistence = NodeInput::Node(from.id.node);
            }
            (NoiseNode::F64(_), NoiseNode::RigidMulti(RigidFractalNode { attenuation, .. }))
                if to.id.input == 5 =>
            {
                *attenuation = NodeInput::Node(from.id.node);
            }
            (_, _) => {
                debug!("Not connecting #{} to #{}", from.id.node, to.id.node);

                return;
            }
        }

        self.updated_node_indices.insert(to.id.node);

        for &remote in &to.remotes {
            debug!("Disconnecting #{} from #{}", remote.node, to.id.node);

            snarl.disconnect(remote, to.id);
        }

        debug!("Connecting #{} to #{}", from.id.node, to.id.node);

        snarl
            .get_node_mut(from.id.node)
            .output_node_indices_mut()
            .insert(to.id.node);
        snarl.connect(from.id, to.id);
    }

    fn title(&mut self, _node: &NoiseNode) -> String {
        unimplemented!()
    }

    fn show_header(
        &mut self,
        node_idx: usize,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) {
        #[cfg(debug_assertions)]
        ui.label(RichText::new(format!("#{node_idx}")).color(Color32::DEBUG_COLOR));

        let node = snarl.get_node_mut(node_idx);

        match node {
            NoiseNode::Abs(_) => {
                ui.label("Abs");
            }
            NoiseNode::Add(_) => {
                ui.label("Add");
            }
            NoiseNode::BasicMulti(FractalNode { source, .. }) => {
                ui.label("Basic Multi");
                self.source_combo_box(ui, source, node_idx);
            }
            NoiseNode::Billow(FractalNode { source, .. }) => {
                ui.label("Billow");
                self.source_combo_box(ui, source, node_idx);
            }
            NoiseNode::F64(ConstantNode { name, value, .. }) => {
                ui.add(egui::TextEdit::singleline(name).desired_width(50.0));

                if ui.add(egui::DragValue::new(value)).changed() {
                    self.updated_node_indices.insert(node_idx);
                }
            }
            NoiseNode::Fbm(FractalNode { source, .. }) => {
                ui.label("fBm");
                self.source_combo_box(ui, source, node_idx);
            }
            NoiseNode::HybridMulti(FractalNode { source, .. }) => {
                ui.label("Hybrid Multi");
                self.source_combo_box(ui, source, node_idx);
            }
            NoiseNode::Min(_) => {
                ui.label("Min");
            }
            NoiseNode::Max(_) => {
                ui.label("Max");
            }
            NoiseNode::Multiply(_) => {
                ui.label("Multiply");
            }
            NoiseNode::Perlin(_) => {
                ui.label("Perlin");
            }
            NoiseNode::Power(_) => {
                ui.label("Power");
            }
            NoiseNode::RigidMulti(RigidFractalNode { source, .. }) => {
                ui.label("Rigid Multi");
                self.source_combo_box(ui, source, node_idx);
            }
            NoiseNode::U32(ConstantNode { name, value, .. }) => {
                ui.add(egui::TextEdit::singleline(name).desired_width(50.0));

                if ui.add(egui::DragValue::new(value)).changed() {
                    self.updated_node_indices.insert(node_idx);
                }
            }
        }

        match node {
            NoiseNode::Abs(AbsNode { input_node_idx, .. }) => {
                *input_node_idx = inputs[0].remotes.get(0).map(|remote| remote.node);
            }
            NoiseNode::Add(CombinerNode {
                input_node_indices, ..
            })
            | NoiseNode::Min(CombinerNode {
                input_node_indices, ..
            })
            | NoiseNode::Max(CombinerNode {
                input_node_indices, ..
            })
            | NoiseNode::Multiply(CombinerNode {
                input_node_indices, ..
            })
            | NoiseNode::Power(CombinerNode {
                input_node_indices, ..
            }) => {
                for (idx, input_node_idx) in input_node_indices.iter_mut().enumerate() {
                    *input_node_idx = inputs[idx].remotes.get(0).map(|remote| remote.node);
                }
            }

            _ => {}
        }

        let output_node_indices = node.output_node_indices_mut();
        if outputs.len() != output_node_indices.len() {
            output_node_indices.clear();

            for remote in outputs.iter().flat_map(|output| output.remotes.iter()) {
                output_node_indices.insert(remote.node);
            }
        }
    }

    fn inputs(&mut self, node: &NoiseNode) -> usize {
        match node {
            NoiseNode::F64(_) | NoiseNode::U32(_) => 0,
            NoiseNode::Abs(_) | NoiseNode::Perlin(_) => 1,
            NoiseNode::Add(_)
            | NoiseNode::Min(_)
            | NoiseNode::Max(_)
            | NoiseNode::Multiply(_)
            | NoiseNode::Power(_) => 2,
            NoiseNode::BasicMulti(_)
            | NoiseNode::Billow(_)
            | NoiseNode::Fbm(_)
            | NoiseNode::HybridMulti(_) => 5,
            NoiseNode::RigidMulti(_) => 6,
        }
    }

    fn outputs(&mut self, _node: &NoiseNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) -> PinInfo {
        // Handle disconnections by resetting node pins to the value of the previous node
        if pin.remotes.is_empty() {
            match (pin.id.input, snarl.get_node(pin.id.node)) {
                (
                    0,
                    &NoiseNode::BasicMulti(FractalNode {
                        seed: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        seed: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        seed: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        seed: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .seed = NodeInput::Value(snarl.get_node(node_idx).as_const_u32().unwrap());
                }
                (
                    0,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        seed: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .seed = NodeInput::Value(snarl.get_node(node_idx).as_const_u32().unwrap());
                }
                (
                    1,
                    &NoiseNode::BasicMulti(FractalNode {
                        octaves: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        octaves: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        octaves: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        octaves: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .octaves =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_u32().unwrap());
                }
                (
                    1,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        octaves: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .octaves =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_u32().unwrap());
                }
                (
                    2,
                    &NoiseNode::BasicMulti(FractalNode {
                        frequency: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        frequency: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        frequency: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        frequency: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .frequency =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                (
                    2,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        frequency: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .frequency =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                (
                    3,
                    &NoiseNode::BasicMulti(FractalNode {
                        lacunarity: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        lacunarity: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        lacunarity: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        lacunarity: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .lacunarity =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                (
                    3,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        lacunarity: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .lacunarity =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                (
                    4,
                    &NoiseNode::BasicMulti(FractalNode {
                        persistence: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        persistence: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        persistence: NodeInput::Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        persistence: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .persistence =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                (
                    4,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        persistence: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .persistence =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                (
                    5,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        attenuation: NodeInput::Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .attenuation =
                        NodeInput::Value(snarl.get_node(node_idx).as_const_f64().unwrap());
                }
                _ => {}
            }
        }

        match (pin.id.input, snarl.get_node_mut(pin.id.node)) {
            (0, NoiseNode::Abs(AbsNode { input_node_idx, .. })) => {
                ui.label("Node");

                if input_node_idx.is_some() {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                }
            }
            (
                0 | 1,
                NoiseNode::Add(CombinerNode {
                    input_node_indices, ..
                })
                | NoiseNode::Min(CombinerNode {
                    input_node_indices, ..
                })
                | NoiseNode::Max(CombinerNode {
                    input_node_indices, ..
                })
                | NoiseNode::Multiply(CombinerNode {
                    input_node_indices, ..
                })
                | NoiseNode::Power(CombinerNode {
                    input_node_indices, ..
                }),
            ) => {
                ui.label("Node");

                if input_node_indices.get(pin.id.input).is_some() {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                }
            }
            (
                0,
                NoiseNode::BasicMulti(FractalNode { seed, .. })
                | NoiseNode::Billow(FractalNode { seed, .. })
                | NoiseNode::Fbm(FractalNode { seed, .. })
                | NoiseNode::HybridMulti(FractalNode { seed, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { seed, .. }),
            ) => {
                ui.label("Seed");

                if let Some(seed) = seed.as_value_mut() {
                    if ui.add(egui::DragValue::new(seed)).changed() {
                        self.updated_node_indices.insert(pin.id.node);
                    }

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }

            (
                1,
                NoiseNode::BasicMulti(FractalNode { octaves, .. })
                | NoiseNode::Billow(FractalNode { octaves, .. })
                | NoiseNode::Fbm(FractalNode { octaves, .. })
                | NoiseNode::HybridMulti(FractalNode { octaves, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { octaves, .. }),
            ) => {
                ui.label("Octaves");

                if let Some(octaves) = octaves.as_value_mut() {
                    if ui
                        .add(
                            egui::DragValue::new(octaves).clamp_range(1..=FractalNode::MAX_OCTAVES),
                        )
                        .changed()
                    {
                        self.updated_node_indices.insert(pin.id.node);
                    }

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }
            (
                2,
                NoiseNode::BasicMulti(FractalNode { frequency, .. })
                | NoiseNode::Billow(FractalNode { frequency, .. })
                | NoiseNode::Fbm(FractalNode { frequency, .. })
                | NoiseNode::HybridMulti(FractalNode { frequency, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { frequency, .. }),
            ) => {
                ui.label("Frequency");

                if let Some(frequency) = frequency.as_value_mut() {
                    self.drag_value_f64(ui, frequency, pin);

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }
            (
                3,
                NoiseNode::BasicMulti(FractalNode { lacunarity, .. })
                | NoiseNode::Billow(FractalNode { lacunarity, .. })
                | NoiseNode::Fbm(FractalNode { lacunarity, .. })
                | NoiseNode::HybridMulti(FractalNode { lacunarity, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { lacunarity, .. }),
            ) => {
                ui.label("Lacunarity");

                if let Some(lacunarity) = lacunarity.as_value_mut() {
                    self.drag_value_f64(ui, lacunarity, pin);

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }
            (
                4,
                NoiseNode::BasicMulti(FractalNode { persistence, .. })
                | NoiseNode::Billow(FractalNode { persistence, .. })
                | NoiseNode::Fbm(FractalNode { persistence, .. })
                | NoiseNode::HybridMulti(FractalNode { persistence, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { persistence, .. }),
            ) => {
                ui.label("Persistence");

                if let Some(persistence) = persistence.as_value_mut() {
                    self.drag_value_f64(ui, persistence, pin);

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }
            (5, NoiseNode::RigidMulti(RigidFractalNode { attenuation, .. })) => {
                ui.label("Attenuation");

                if let Some(attenuation) = attenuation.as_value_mut() {
                    self.drag_value_f64(ui, attenuation, pin);

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }
            (0, NoiseNode::Perlin(PerlinNode { seed, .. })) => {
                ui.label("Seed");

                if let Some(seed) = seed.as_value_mut() {
                    if ui.add(egui::DragValue::new(seed)).changed() {
                        self.updated_node_indices.insert(pin.id.node);
                    }

                    PinInfo::circle().with_fill(egui::Color32::GRAY)
                } else {
                    PinInfo::circle().with_fill(egui::Color32::GREEN)
                }
            }
            _ => unreachable!(),
        }
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) -> PinInfo {
        let node = snarl.get_node(pin.id.node);

        if let Some(texture) = node.image().and_then(|image| image.texture.as_ref()) {
            ui.image((texture.id(), texture.size_vec2() * scale));
        }

        match snarl.get_node(pin.id.node) {
            NoiseNode::Abs(_)
            | NoiseNode::Add(_)
            | NoiseNode::BasicMulti(_)
            | NoiseNode::Billow(_)
            | NoiseNode::Fbm(_)
            | NoiseNode::HybridMulti(_)
            | NoiseNode::Min(_)
            | NoiseNode::Max(_)
            | NoiseNode::Multiply(_)
            | NoiseNode::Perlin(_)
            | NoiseNode::Power(_)
            | NoiseNode::RigidMulti(_) => PinInfo::square().with_fill(egui::Color32::GOLD),
            NoiseNode::F64(_) => {
                ui.label("f64");
                PinInfo::square().with_fill(egui::Color32::GOLD)
            }
            NoiseNode::U32(_) => {
                ui.label("u32");
                PinInfo::square().with_fill(egui::Color32::GOLD)
            }
        }
    }

    fn input_color(
        &mut self,
        _pin: &InPin,
        _style: &egui::Style,
        _snarl: &mut Snarl<NoiseNode>,
    ) -> egui::Color32 {
        unimplemented!()
    }

    fn output_color(
        &mut self,
        _pin: &OutPin,
        _style: &egui::Style,
        _snarl: &mut Snarl<NoiseNode>,
    ) -> egui::Color32 {
        unimplemented!()
    }

    fn graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) {
        ui.label("Add node");

        ui.separator();
        ui.label("Combiners");

        if ui.button("Add").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Add(Default::default())));
            ui.close_menu();
        }

        if ui.button("Min").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Min(Default::default())));
            ui.close_menu();
        }

        if ui.button("Max").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Max(Default::default())));
            ui.close_menu();
        }

        if ui.button("Multiply").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Multiply(Default::default())));
            ui.close_menu();
        }

        if ui.button("Power").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Power(Default::default())));
            ui.close_menu();
        }

        ui.separator();
        ui.label("Generators");

        if ui.button("Perlin").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Perlin(Default::default())));
            ui.close_menu();
        }

        ui.separator();
        ui.label("Fractals");

        if ui.button("Basic Multi").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::BasicMulti(Default::default())));
            ui.close_menu();
        }

        if ui.button("Hybrid Multi").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::HybridMulti(Default::default())));
            ui.close_menu();
        }

        if ui.button("Rigid Multi").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::RigidMulti(Default::default())));
            ui.close_menu();
        }

        if ui.button("fBm").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Fbm(Default::default())));
            ui.close_menu();
        }

        if ui.button("Billow").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Billow(Default::default())));
            ui.close_menu();
        }

        ui.separator();
        ui.label("Modifiers");

        if ui.button("Abs").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Abs(Default::default())));
            ui.close_menu();
        }

        ui.separator();
        ui.label("Constants");

        if ui.button("f64").clicked() {
            snarl.insert_node(pos, NoiseNode::F64(Default::default()));
            ui.close_menu();
        }

        if ui.button("u32").clicked() {
            snarl.insert_node(pos, NoiseNode::U32(Default::default()));
            ui.close_menu();
        }
    }

    fn node_menu(
        &mut self,
        node_idx: usize,
        inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            self.removed_node_indices.insert(node_idx);

            for input in inputs {
                snarl
                    .get_node_mut(input.id.node)
                    .output_node_indices_mut()
                    .remove(&node_idx);
            }

            snarl.remove_node(node_idx);
            ui.close_menu();
        }
    }
}
