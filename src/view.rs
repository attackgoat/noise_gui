use egui::{Color32, RichText};

use crate::node::AbsNode;

use {
    super::node::{ConstantNode, NodeInput, NoiseNode, PerlinNode},
    egui_snarl::{
        ui::{InPin, OutPin, PinInfo, SnarlViewer},
        Snarl,
    },
    std::collections::HashSet,
};

pub struct Viewer<'a> {
    pub removed_node_indices: &'a mut HashSet<usize>,
    pub updated_node_indices: &'a mut HashSet<usize>,
}

impl<'a> SnarlViewer<NoiseNode> for Viewer<'a> {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NoiseNode>) {
        let from_node = snarl.get_node(from.id.node).clone();
        let to_node = snarl.get_node_mut(to.id.node);

        match (from_node, to_node) {
            (NoiseNode::Abs(_), _) => return,
            (NoiseNode::F64(_), _) => return,
            (NoiseNode::Perlin(_), NoiseNode::Abs(_)) => {
                self.updated_node_indices.insert(to.id.node);
            }
            (NoiseNode::Perlin(_), _) => return,
            (NoiseNode::U32(_), NoiseNode::Perlin(PerlinNode { seed, .. })) => {
                *seed = NodeInput::Node(from.id.node);
                self.updated_node_indices.insert(to.id.node);
            }
            (NoiseNode::U32(_), _) => {}
        }

        for &remote in &to.remotes {
            snarl.disconnect(remote, to.id);
        }

        snarl
            .get_node_mut(from.id.node)
            .output_node_indices_mut()
            .insert(to.id.node);
        snarl.connect(from.id, to.id);

        self.updated_node_indices.insert(to.id.node);
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
            NoiseNode::Abs(AbsNode { input_node_idx, .. }) => {
                ui.label("Abs");

                if inputs[0].remotes.is_empty() {
                    *input_node_idx = None;
                } else {
                    *input_node_idx = Some(inputs[0].remotes[0].node);
                }
            }
            NoiseNode::F64(ConstantNode { name, value, .. }) => {
                ui.add(egui::TextEdit::singleline(name).desired_width(50.0));

                if ui.add(egui::DragValue::new(value)).changed() {
                    self.updated_node_indices.insert(node_idx);
                }
            }
            NoiseNode::Perlin(_) => {
                ui.label("Perlin");
            }
            NoiseNode::U32(ConstantNode { name, value, .. }) => {
                ui.add(egui::TextEdit::singleline(name).desired_width(50.0));

                if ui.add(egui::DragValue::new(value)).changed() {
                    self.updated_node_indices.insert(node_idx);
                }
            }
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
        match snarl.get_node_mut(pin.id.node) {
            NoiseNode::Abs(_) => {
                ui.label("Node");
                match *pin.remotes.as_slice() {
                    [] => PinInfo::circle().with_fill(egui::Color32::GRAY),
                    [remote] => match *snarl.get_node(remote.node) {
                        NoiseNode::Abs(_) | NoiseNode::Perlin(_) => {
                            PinInfo::circle().with_fill(egui::Color32::GREEN)
                        }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            }
            NoiseNode::Perlin(PerlinNode { seed, .. }) => {
                ui.label("Seed");
                match *pin.remotes.as_slice() {
                    [] => {
                        if let NodeInput::Value(value) = seed {
                            if ui.add(egui::DragValue::new(value)).changed() {
                                self.updated_node_indices.insert(pin.id.node);
                            }
                        }

                        PinInfo::circle().with_fill(egui::Color32::GRAY)
                    }
                    [remote] => match *snarl.get_node(remote.node) {
                        NoiseNode::U32(_) => PinInfo::circle().with_fill(egui::Color32::GREEN),
                        _ => unreachable!(),
                    },
                    _ => unreachable!("Perlin input has only one wire"),
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

        if let Some(texture) = node.texture_handle() {
            ui.image((texture.id(), texture.size_vec2() * scale));
        }

        match snarl.get_node(pin.id.node) {
            NoiseNode::Abs(_) | NoiseNode::Perlin(_) => {
                PinInfo::square().with_fill(egui::Color32::GOLD)
            }
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

        if ui.button("Abs").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Abs(Default::default())));
            ui.close_menu();
        }

        if ui.button("f64").clicked() {
            snarl.insert_node(pos, NoiseNode::F64(Default::default()));
            ui.close_menu();
        }

        if ui.button("Perlin").clicked() {
            self.updated_node_indices
                .insert(snarl.insert_node(pos, NoiseNode::Perlin(Default::default())));
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
