use {
    super::{
        expr::{DistanceFunction, OpType, ReturnType, SourceType, MAX_FRACTAL_OCTAVES},
        node::{
            BlendNode, CheckerboardNode, ClampNode, CombinerNode, ConstantOpNode, ControlPointNode,
            CurveNode, CylindersNode, DisplaceNode, ExponentNode, FractalNode, GeneratorNode,
            NodeValue::{Node, Value},
            NoiseNode, RigidFractalNode, ScaleBiasNode, SelectNode, TerraceNode, TransformNode,
            TurbulenceNode, UnaryNode, WorleyNode,
        },
    },
    egui::{
        epaint::PathShape, vec2, Align, Color32, ComboBox, DragValue, Layout, Pos2, Shape, Stroke,
        Style, TextEdit, Ui, Vec2,
    },
    egui_snarl::{
        ui::{InPin, OutPin, PinInfo, SnarlViewer},
        Snarl,
    },
    log::debug,
    std::{cell::RefCell, collections::HashSet},
};

#[cfg(debug_assertions)]
use egui::RichText;

#[cfg(not(target_arch = "wasm32"))]
use super::app::App;

pub struct Viewer<'a> {
    pub removed_node_indices: &'a mut HashSet<usize>,
    pub updated_node_indices: &'a mut HashSet<usize>,
}

impl<'a> Viewer<'a> {
    const AXES: [&'static str; 4] = ["X", "Y", "Z", "W"];

    fn control_point_pin_info(is_input: bool, filled: bool) -> PinInfo {
        let fill = Color32::from_rgb(132, 80, 24);

        Self::scalar_pin_info(is_input, filled, fill)
    }

    // TODO: Make generic (see other combo box functions)
    fn distance_fn_combo_box(
        &mut self,
        ui: &mut Ui,
        distance_fn: &mut DistanceFunction,
        node_idx: usize,
    ) {
        ComboBox::from_id_source(0)
            .selected_text(format!("{distance_fn:?}"))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for value in [
                    DistanceFunction::Chebyshev,
                    DistanceFunction::Euclidean,
                    DistanceFunction::EuclideanSquared,
                    DistanceFunction::Manhattan,
                ] {
                    if ui
                        .selectable_value(distance_fn, value, format!("{value:?}"))
                        .changed()
                    {
                        self.updated_node_indices.insert(node_idx);
                    }
                }
            });
    }

    fn drag_value_f64(&mut self, ui: &mut Ui, scale: f32, value: &mut f64, node_idx: usize) {
        ui.with_layout(
            Layout::right_to_left(Align::Min).with_cross_align(Align::Center),
            |ui| {
                ui.set_height(16.0 * scale);
                if ui
                    .add(
                        DragValue::new(value)
                            .min_decimals(2)
                            .max_decimals(2)
                            .speed(0.01),
                    )
                    .changed()
                {
                    self.updated_node_indices.insert(node_idx);
                }
            },
        );
    }

    fn drag_value_octaves(&mut self, ui: &mut Ui, scale: f32, value: &mut u32, node_idx: usize) {
        ui.with_layout(
            Layout::right_to_left(Align::Min).with_cross_align(Align::Center),
            |ui| {
                ui.set_height(16.0 * scale);
                if ui
                    .add(DragValue::new(value).clamp_range(1..=MAX_FRACTAL_OCTAVES))
                    .changed()
                {
                    self.updated_node_indices.insert(node_idx);
                }
            },
        );
    }

    fn drag_value_u32(&mut self, ui: &mut Ui, scale: f32, value: &mut u32, node_idx: usize) {
        ui.with_layout(
            Layout::right_to_left(Align::Min).with_cross_align(Align::Center),
            |ui| {
                ui.set_height(16.0 * scale);
                if ui.add(DragValue::new(value)).changed() {
                    self.updated_node_indices.insert(node_idx);
                }
            },
        );
    }

    fn f64_pin_info(is_input: bool, filled: bool) -> PinInfo {
        let fill = Color32::from_rgb(128, 64, 192);

        Self::scalar_pin_info(is_input, filled, fill)
    }

    fn image_pin_info(is_input: bool, filled: bool) -> PinInfo {
        PinInfo::default()
            .with_fill(Color32::from_gray(192))
            .with_stroke(Stroke::new(1.5, Color32::from_white_alpha(192)))
            .with_shape(egui_snarl::ui::PinShape::Custom(Box::new(
                move |painter, rect, _fill, stroke| {
                    const S: f32 = 1.0;
                    const A: Vec2 = vec2(0.2 * S, 0.7 * S);
                    const B: Vec2 = vec2(0.2 * S, -0.7 * S);
                    const C: Vec2 = vec2(0.9 * S, -0.7 * S);
                    const D: Vec2 = vec2(1.5 * S, 0.0 * S);
                    const E: Vec2 = vec2(0.9 * S, 0.7 * S);

                    let mut pos = rect.min;
                    let size = rect.size();
                    pos.y += 0.5 * size.y;

                    if is_input {
                        pos.x -= 0.6 * size.x;
                    }

                    let points = vec![
                        pos + A * size,
                        pos + B * size,
                        pos + C * size,
                        pos + D * size,
                        pos + E * size,
                    ];

                    // The fill set on the pin sets the wire color - I want a different fill here
                    let fill = if filled {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(16)
                    };

                    painter.add(Shape::Path(PathShape {
                        points,
                        closed: true,
                        fill,
                        stroke,
                    }));
                },
            )))
    }

    fn operation_pin_info(is_input: bool, filled: bool) -> PinInfo {
        let fill = Color32::from_gray(127);

        Self::scalar_pin_info(is_input, filled, fill)
    }

    // TODO: Make generic (see other combo box functions)
    fn return_ty_combo_box(&mut self, ui: &mut Ui, return_ty: &mut ReturnType, node_idx: usize) {
        ComboBox::from_id_source(1)
            .selected_text(format!("{return_ty:?}"))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for value in [ReturnType::Distance, ReturnType::Value] {
                    if ui
                        .selectable_value(return_ty, value, format!("{value:?}"))
                        .changed()
                    {
                        self.updated_node_indices.insert(node_idx);
                    }
                }
            });
    }

    // TODO: Make generic (see other combo box functions)
    fn source_ty_combo_box(&mut self, ui: &mut Ui, source: &mut SourceType, node_idx: usize) {
        ComboBox::from_id_source(0)
            .selected_text(format!("{source:?}"))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for value in [
                    SourceType::OpenSimplex,
                    SourceType::Perlin,
                    SourceType::PerlinSurflet,
                    SourceType::Simplex,
                    SourceType::SuperSimplex,
                    SourceType::Value,
                    SourceType::Worley,
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

    fn scalar_pin_info(is_input: bool, filled: bool, fill: Color32) -> PinInfo {
        let (r, g, b, _) = fill.to_tuple();

        PinInfo::default()
            .with_fill(fill)
            .with_stroke(Stroke::new(
                1.5,
                Color32::from_rgba_unmultiplied(r, g, b, 192),
            ))
            .with_shape(egui_snarl::ui::PinShape::Custom(Box::new(
                move |painter, rect, _fill, stroke| {
                    const S: f32 = 1.0;
                    const A: Vec2 = vec2(0.64 * S, 0.07 * S);
                    const B: Vec2 = vec2(0.64 * S, -0.07 * S);
                    const C: Vec2 = vec2(0.72 * S, 0.0 * S);

                    let mut pos = rect.min;
                    let size = rect.size();
                    pos.y += 0.5 * size.y;

                    if !is_input {
                        pos.x += 0.6 * size.x;
                    }

                    let points = vec![pos + A * size, pos + B * size, pos + C * size];

                    painter.add(Shape::Path(PathShape {
                        points,
                        closed: true,
                        fill,
                        stroke,
                    }));

                    let radius = 0.5 * size.x;

                    painter.add(if filled {
                        Shape::circle_filled(pos, radius, fill)
                    } else {
                        Shape::circle_stroke(pos, radius, stroke)
                    });
                },
            )))
    }

    fn u32_pin_info(is_input: bool, filled: bool) -> PinInfo {
        let fill = Color32::from_rgb(64, 192, 176);

        Self::scalar_pin_info(is_input, filled, fill)
    }
}

impl<'a> SnarlViewer<NoiseNode> for Viewer<'a> {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NoiseNode>) {
        // Make sure this connection is not to the same node
        if from.id.node == to.id.node {
            debug!("Not connecting #{} to #{} (Same)", from.id.node, to.id.node);

            return;
        }

        // Make sure this connection does not create a cyclic node graph
        {
            thread_local! {
                static NODE_INDICES: RefCell<Option<Vec<usize>>> = RefCell::new(Some(Default::default()));
            }

            let mut node_indices = NODE_INDICES.take().unwrap();
            node_indices.push(to.id.node);

            while let Some(node_idx) = node_indices.pop() {
                for node_idx in snarl
                    .get_node(node_idx)
                    .output_node_indices()
                    .iter()
                    .copied()
                {
                    if node_idx == from.id.node {
                        node_indices.clear();
                        NODE_INDICES.set(Some(node_indices));

                        debug!(
                            "Not connecting #{} to #{} (Cyclic)",
                            from.id.node, to.id.node
                        );

                        // We found a cycle
                        return;
                    }

                    node_indices.push(node_idx);
                }
            }

            node_indices.clear();
            NODE_INDICES.set(Some(node_indices));
        }

        // Handle operation nodes (automatically change types based on inputs/ouputs)
        {
            if let NoiseNode::Operation(_) = snarl.get_node(from.id.node) {
                match (to.id.input, snarl.get_node(to.id.node)) {
                    (
                        0,
                        NoiseNode::Abs(_)
                        | NoiseNode::Clamp(_)
                        | NoiseNode::ControlPoint(_)
                        | NoiseNode::Curve(_)
                        | NoiseNode::Cylinders(_)
                        | NoiseNode::Displace(_)
                        | NoiseNode::Exponent(_)
                        | NoiseNode::Negate(_)
                        | NoiseNode::RotatePoint(_)
                        | NoiseNode::ScaleBias(_)
                        | NoiseNode::ScalePoint(_)
                        | NoiseNode::Terrace(_)
                        | NoiseNode::TranslatePoint(_)
                        | NoiseNode::Turbulence(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        0,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Checkerboard(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_)
                        | NoiseNode::OpenSimplex(_)
                        | NoiseNode::Perlin(_)
                        | NoiseNode::PerlinSurflet(_)
                        | NoiseNode::RigidMulti(_)
                        | NoiseNode::Simplex(_)
                        | NoiseNode::SuperSimplex(_)
                        | NoiseNode::Value(_)
                        | NoiseNode::Worley(_),
                    ) => {
                        NoiseNode::propagate_u32_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        0 | 1,
                        NoiseNode::Add(_)
                        | NoiseNode::Blend(_)
                        | NoiseNode::F64Operation(_)
                        | NoiseNode::Min(_)
                        | NoiseNode::Max(_)
                        | NoiseNode::Multiply(_)
                        | NoiseNode::Power(_)
                        | NoiseNode::Select(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (0 | 1, NoiseNode::U32Operation(_)) => {
                        NoiseNode::propagate_u32_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        1,
                        NoiseNode::Clamp(_)
                        | NoiseNode::ControlPoint(_)
                        | NoiseNode::Exponent(_)
                        | NoiseNode::ScaleBias(_)
                        | NoiseNode::Worley(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        1,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_)
                        | NoiseNode::RigidMulti(_)
                        | NoiseNode::Turbulence(_),
                    ) => {
                        NoiseNode::propagate_u32_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        1..=4,
                        NoiseNode::Displace(_)
                        | NoiseNode::RotatePoint(_)
                        | NoiseNode::ScalePoint(_)
                        | NoiseNode::TranslatePoint(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        2,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Blend(_)
                        | NoiseNode::Clamp(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_)
                        | NoiseNode::RigidMulti(_)
                        | NoiseNode::ScaleBias(_)
                        | NoiseNode::Select(_)
                        | NoiseNode::Turbulence(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (
                        3,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_)
                        | NoiseNode::RigidMulti(_)
                        | NoiseNode::Select(_)
                        | NoiseNode::Turbulence(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }

                    (
                        4,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_)
                        | NoiseNode::RigidMulti(_)
                        | NoiseNode::Select(_),
                    ) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (4, NoiseNode::Turbulence(_)) => {
                        NoiseNode::propagate_u32_from_tuple_op(from.id.node, snarl);
                    }
                    (5, NoiseNode::RigidMulti(_) | NoiseNode::Select(_)) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    (_, NoiseNode::Terrace(_)) => {
                        NoiseNode::propagate_f64_from_tuple_op(from.id.node, snarl);
                    }
                    _ => (),
                }
            }

            if let NoiseNode::Operation(_) = snarl.get_node(to.id.node) {
                match snarl.get_node(from.id.node) {
                    NoiseNode::Abs(_)
                    | NoiseNode::Add(_)
                    | NoiseNode::BasicMulti(_)
                    | NoiseNode::Billow(_)
                    | NoiseNode::Blend(_)
                    | NoiseNode::Clamp(_)
                    | NoiseNode::Checkerboard(_)
                    | NoiseNode::ControlPoint(_)
                    | NoiseNode::Curve(_)
                    | NoiseNode::Cylinders(_)
                    | NoiseNode::Displace(_)
                    | NoiseNode::Exponent(_)
                    | NoiseNode::Fbm(_)
                    | NoiseNode::HybridMulti(_)
                    | NoiseNode::Max(_)
                    | NoiseNode::Min(_)
                    | NoiseNode::Multiply(_)
                    | NoiseNode::Negate(_)
                    | NoiseNode::OpenSimplex(_)
                    | NoiseNode::Operation(_)
                    | NoiseNode::Perlin(_)
                    | NoiseNode::PerlinSurflet(_)
                    | NoiseNode::Power(_)
                    | NoiseNode::RigidMulti(_)
                    | NoiseNode::RotatePoint(_)
                    | NoiseNode::ScaleBias(_)
                    | NoiseNode::ScalePoint(_)
                    | NoiseNode::Select(_)
                    | NoiseNode::Simplex(_)
                    | NoiseNode::SuperSimplex(_)
                    | NoiseNode::Terrace(_)
                    | NoiseNode::TranslatePoint(_)
                    | NoiseNode::Turbulence(_)
                    | NoiseNode::Value(_)
                    | NoiseNode::Worley(_) => (),
                    NoiseNode::F64(_) | NoiseNode::F64Operation(_) => {
                        NoiseNode::propagate_f64_from_tuple_op(to.id.node, snarl)
                    }
                    NoiseNode::U32(_) | NoiseNode::U32Operation(_) => {
                        NoiseNode::propagate_u32_from_tuple_op(to.id.node, snarl)
                    }
                }
            }
        }
        let from_node = snarl.get_node(from.id.node).clone();
        let to_node = snarl.get_node_mut(to.id.node);

        match (from_node, to.id.input, to_node) {
            (
                NoiseNode::Abs(_)
                | NoiseNode::Add(_)
                | NoiseNode::BasicMulti(_)
                | NoiseNode::Billow(_)
                | NoiseNode::Blend(_)
                | NoiseNode::Checkerboard(_)
                | NoiseNode::Clamp(_)
                | NoiseNode::ControlPoint(_)
                | NoiseNode::Curve(_)
                | NoiseNode::Cylinders(_)
                | NoiseNode::Displace(_)
                | NoiseNode::Exponent(_)
                | NoiseNode::F64(_)
                | NoiseNode::F64Operation(_)
                | NoiseNode::Fbm(_)
                | NoiseNode::HybridMulti(_)
                | NoiseNode::Max(_)
                | NoiseNode::Min(_)
                | NoiseNode::Multiply(_)
                | NoiseNode::Negate(_)
                | NoiseNode::OpenSimplex(_)
                | NoiseNode::Perlin(_)
                | NoiseNode::PerlinSurflet(_)
                | NoiseNode::Power(_)
                | NoiseNode::RigidMulti(_)
                | NoiseNode::RotatePoint(_)
                | NoiseNode::ScaleBias(_)
                | NoiseNode::ScalePoint(_)
                | NoiseNode::Select(_)
                | NoiseNode::Simplex(_)
                | NoiseNode::SuperSimplex(_)
                | NoiseNode::Terrace(_)
                | NoiseNode::TranslatePoint(_)
                | NoiseNode::Turbulence(_)
                | NoiseNode::Value(_)
                | NoiseNode::Worley(_),
                0,
                NoiseNode::Abs(UnaryNode { input_node_idx, .. })
                | NoiseNode::Clamp(ClampNode { input_node_idx, .. })
                | NoiseNode::Curve(CurveNode { input_node_idx, .. })
                | NoiseNode::Displace(DisplaceNode { input_node_idx, .. })
                | NoiseNode::Exponent(ExponentNode { input_node_idx, .. })
                | NoiseNode::Negate(UnaryNode { input_node_idx, .. })
                | NoiseNode::RotatePoint(TransformNode { input_node_idx, .. })
                | NoiseNode::ScaleBias(ScaleBiasNode { input_node_idx, .. })
                | NoiseNode::ScalePoint(TransformNode { input_node_idx, .. })
                | NoiseNode::Terrace(TerraceNode { input_node_idx, .. })
                | NoiseNode::TranslatePoint(TransformNode { input_node_idx, .. })
                | NoiseNode::Turbulence(TurbulenceNode { input_node_idx, .. }),
            ) => {
                *input_node_idx = Some(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 0, NoiseNode::ControlPoint(node)) => {
                node.input = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 0, NoiseNode::Cylinders(node)) => {
                node.frequency = Node(from.id.node);
            }
            (NoiseNode::U32(_) | NoiseNode::U32Operation(_), 0, NoiseNode::Checkerboard(node)) => {
                node.size = Node(from.id.node);
            }
            (
                NoiseNode::U32(_) | NoiseNode::U32Operation(_),
                0,
                NoiseNode::BasicMulti(FractalNode { seed, .. })
                | NoiseNode::Billow(FractalNode { seed, .. })
                | NoiseNode::Fbm(FractalNode { seed, .. })
                | NoiseNode::HybridMulti(FractalNode { seed, .. })
                | NoiseNode::OpenSimplex(GeneratorNode { seed, .. })
                | NoiseNode::Perlin(GeneratorNode { seed, .. })
                | NoiseNode::PerlinSurflet(GeneratorNode { seed, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { seed, .. })
                | NoiseNode::Simplex(GeneratorNode { seed, .. })
                | NoiseNode::SuperSimplex(GeneratorNode { seed, .. })
                | NoiseNode::Value(GeneratorNode { seed, .. })
                | NoiseNode::Worley(WorleyNode { seed, .. }),
            ) => {
                *seed = Node(from.id.node);
            }
            (
                NoiseNode::F64(_) | NoiseNode::F64Operation(_),
                0 | 1,
                NoiseNode::F64Operation(node),
            ) => {
                node.inputs[to.id.input] = Node(from.id.node);
            }
            (NoiseNode::Operation(_), 0 | 1, NoiseNode::Operation(node)) => {
                node.inputs[to.id.input] = Node(from.id.node);
            }
            (
                NoiseNode::U32(_) | NoiseNode::U32Operation(_),
                0 | 1,
                NoiseNode::U32Operation(node),
            ) => {
                node.inputs[to.id.input] = Node(from.id.node);
            }
            (
                NoiseNode::Abs(_)
                | NoiseNode::Add(_)
                | NoiseNode::BasicMulti(_)
                | NoiseNode::Billow(_)
                | NoiseNode::Blend(_)
                | NoiseNode::Checkerboard(_)
                | NoiseNode::Clamp(_)
                | NoiseNode::ControlPoint(_)
                | NoiseNode::Curve(_)
                | NoiseNode::Cylinders(_)
                | NoiseNode::Displace(_)
                | NoiseNode::Exponent(_)
                | NoiseNode::F64(_)
                | NoiseNode::F64Operation(_)
                | NoiseNode::Fbm(_)
                | NoiseNode::HybridMulti(_)
                | NoiseNode::Max(_)
                | NoiseNode::Min(_)
                | NoiseNode::Multiply(_)
                | NoiseNode::Negate(_)
                | NoiseNode::OpenSimplex(_)
                | NoiseNode::Perlin(_)
                | NoiseNode::PerlinSurflet(_)
                | NoiseNode::Power(_)
                | NoiseNode::RigidMulti(_)
                | NoiseNode::RotatePoint(_)
                | NoiseNode::ScaleBias(_)
                | NoiseNode::ScalePoint(_)
                | NoiseNode::Select(_)
                | NoiseNode::Simplex(_)
                | NoiseNode::SuperSimplex(_)
                | NoiseNode::Terrace(_)
                | NoiseNode::TranslatePoint(_)
                | NoiseNode::Turbulence(_)
                | NoiseNode::Value(_)
                | NoiseNode::Worley(_),
                0 | 1,
                NoiseNode::Add(CombinerNode {
                    input_node_indices, ..
                })
                | NoiseNode::Blend(BlendNode {
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
                })
                | NoiseNode::Select(SelectNode {
                    input_node_indices, ..
                }),
            ) => {
                input_node_indices[to.id.input] = Some(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 1, NoiseNode::Clamp(node)) => {
                node.lower_bound = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 1, NoiseNode::ControlPoint(node)) => {
                node.output = Node(from.id.node);
            }
            (
                NoiseNode::U32(_) | NoiseNode::U32Operation(_),
                1,
                NoiseNode::BasicMulti(FractalNode { octaves, .. })
                | NoiseNode::Billow(FractalNode { octaves, .. })
                | NoiseNode::Fbm(FractalNode { octaves, .. })
                | NoiseNode::HybridMulti(FractalNode { octaves, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { octaves, .. }),
            ) => {
                *octaves = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 1, NoiseNode::Exponent(node)) => {
                node.exponent = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 1, NoiseNode::ScaleBias(node)) => {
                node.scale = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 1, NoiseNode::Worley(node)) => {
                node.frequency = Node(from.id.node);
            }
            (NoiseNode::U32(_) | NoiseNode::U32Operation(_), 1, NoiseNode::Turbulence(node)) => {
                node.seed = Node(from.id.node);
            }
            (
                NoiseNode::Abs(_)
                | NoiseNode::Add(_)
                | NoiseNode::BasicMulti(_)
                | NoiseNode::Billow(_)
                | NoiseNode::Blend(_)
                | NoiseNode::Checkerboard(_)
                | NoiseNode::Clamp(_)
                | NoiseNode::ControlPoint(_)
                | NoiseNode::Curve(_)
                | NoiseNode::Cylinders(_)
                | NoiseNode::Displace(_)
                | NoiseNode::Exponent(_)
                | NoiseNode::F64(_)
                | NoiseNode::F64Operation(_)
                | NoiseNode::Fbm(_)
                | NoiseNode::HybridMulti(_)
                | NoiseNode::Max(_)
                | NoiseNode::Min(_)
                | NoiseNode::Multiply(_)
                | NoiseNode::Negate(_)
                | NoiseNode::OpenSimplex(_)
                | NoiseNode::Perlin(_)
                | NoiseNode::PerlinSurflet(_)
                | NoiseNode::Power(_)
                | NoiseNode::RigidMulti(_)
                | NoiseNode::RotatePoint(_)
                | NoiseNode::ScaleBias(_)
                | NoiseNode::ScalePoint(_)
                | NoiseNode::Select(_)
                | NoiseNode::Simplex(_)
                | NoiseNode::SuperSimplex(_)
                | NoiseNode::Terrace(_)
                | NoiseNode::TranslatePoint(_)
                | NoiseNode::Turbulence(_)
                | NoiseNode::Value(_)
                | NoiseNode::Worley(_),
                1..=4,
                NoiseNode::Displace(node),
            ) => {
                node.axes[to.id.input - 1] = Some(from.id.node);
            }
            (
                NoiseNode::F64(_) | NoiseNode::F64Operation(_),
                1..=4,
                NoiseNode::RotatePoint(node)
                | NoiseNode::ScalePoint(node)
                | NoiseNode::TranslatePoint(node),
            ) => {
                node.axes[to.id.input - 1] = Node(from.id.node);
            }
            (
                NoiseNode::Abs(_)
                | NoiseNode::Add(_)
                | NoiseNode::BasicMulti(_)
                | NoiseNode::Billow(_)
                | NoiseNode::Blend(_)
                | NoiseNode::Checkerboard(_)
                | NoiseNode::Clamp(_)
                | NoiseNode::ControlPoint(_)
                | NoiseNode::Curve(_)
                | NoiseNode::Cylinders(_)
                | NoiseNode::Displace(_)
                | NoiseNode::Exponent(_)
                | NoiseNode::F64(_)
                | NoiseNode::F64Operation(_)
                | NoiseNode::Fbm(_)
                | NoiseNode::HybridMulti(_)
                | NoiseNode::Max(_)
                | NoiseNode::Min(_)
                | NoiseNode::Multiply(_)
                | NoiseNode::Negate(_)
                | NoiseNode::OpenSimplex(_)
                | NoiseNode::Perlin(_)
                | NoiseNode::PerlinSurflet(_)
                | NoiseNode::Power(_)
                | NoiseNode::RigidMulti(_)
                | NoiseNode::RotatePoint(_)
                | NoiseNode::ScaleBias(_)
                | NoiseNode::ScalePoint(_)
                | NoiseNode::Select(_)
                | NoiseNode::Simplex(_)
                | NoiseNode::SuperSimplex(_)
                | NoiseNode::Terrace(_)
                | NoiseNode::TranslatePoint(_)
                | NoiseNode::Turbulence(_)
                | NoiseNode::Value(_)
                | NoiseNode::Worley(_),
                2,
                NoiseNode::Blend(BlendNode {
                    control_node_idx, ..
                })
                | NoiseNode::Select(SelectNode {
                    control_node_idx, ..
                }),
            ) => {
                *control_node_idx = Some(from.id.node);
            }
            (
                NoiseNode::F64(_) | NoiseNode::F64Operation(_),
                2,
                NoiseNode::BasicMulti(FractalNode { frequency, .. })
                | NoiseNode::Billow(FractalNode { frequency, .. })
                | NoiseNode::Fbm(FractalNode { frequency, .. })
                | NoiseNode::HybridMulti(FractalNode { frequency, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { frequency, .. })
                | NoiseNode::Turbulence(TurbulenceNode { frequency, .. }),
            ) => {
                *frequency = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 2, NoiseNode::Clamp(node)) => {
                node.upper_bound = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 2, NoiseNode::ScaleBias(node)) => {
                node.bias = Node(from.id.node);
            }
            (
                NoiseNode::F64(_) | NoiseNode::F64Operation(_),
                3,
                NoiseNode::BasicMulti(FractalNode { lacunarity, .. })
                | NoiseNode::Billow(FractalNode { lacunarity, .. })
                | NoiseNode::Fbm(FractalNode { lacunarity, .. })
                | NoiseNode::HybridMulti(FractalNode { lacunarity, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { lacunarity, .. }),
            ) => {
                *lacunarity = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 3, NoiseNode::Select(node)) => {
                node.lower_bound = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 3, NoiseNode::Turbulence(node)) => {
                node.power = Node(from.id.node);
            }
            (
                NoiseNode::F64(_) | NoiseNode::F64Operation(_),
                4,
                NoiseNode::BasicMulti(FractalNode { persistence, .. })
                | NoiseNode::Billow(FractalNode { persistence, .. })
                | NoiseNode::Fbm(FractalNode { persistence, .. })
                | NoiseNode::HybridMulti(FractalNode { persistence, .. })
                | NoiseNode::RigidMulti(RigidFractalNode { persistence, .. }),
            ) => {
                *persistence = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 4, NoiseNode::Select(node)) => {
                node.upper_bound = Node(from.id.node);
            }
            (NoiseNode::U32(_) | NoiseNode::U32Operation(_), 4, NoiseNode::Turbulence(node)) => {
                node.roughness = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 5, NoiseNode::RigidMulti(node)) => {
                node.attenuation = Node(from.id.node);
            }
            (NoiseNode::F64(_) | NoiseNode::F64Operation(_), 5, NoiseNode::Select(node)) => {
                node.falloff = Node(from.id.node);
            }
            (NoiseNode::ControlPoint(_), to_input, NoiseNode::Curve(node)) => {
                let control_point_idx = to_input - 1;

                while node.control_point_node_indices.len() <= control_point_idx {
                    node.control_point_node_indices.push(None);
                }

                node.control_point_node_indices[control_point_idx] = Some(from.id.node);
            }
            (
                NoiseNode::F64(_) | NoiseNode::F64Operation(_),
                to_input,
                NoiseNode::Terrace(node),
            ) => {
                let control_point_idx = to_input - 1;

                while node.control_point_node_indices.len() <= control_point_idx {
                    node.control_point_node_indices.push(None);
                }

                node.control_point_node_indices[control_point_idx] = Some(from.id.node);
            }
            (..) => {
                debug!(
                    "Not connecting #{} to #{} (Incompatible)",
                    from.id.node, to.id.node
                );

                return;
            }
        }

        self.updated_node_indices.insert(to.id.node);

        for &remote in &to.remotes {
            debug!("Disconnecting #{} from #{}", remote.node, to.id.node);

            snarl
                .get_node_mut(remote.node)
                .output_node_indices_mut()
                .remove(&to.id.node);
            snarl.disconnect(remote, to.id);
            NoiseNode::propagate_tuple_from_f64_op(remote.node, snarl);
            NoiseNode::propagate_tuple_from_u32_op(remote.node, snarl);
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
        _inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) {
        #[cfg(debug_assertions)]
        ui.label(RichText::new(format!("#{node_idx}")).color(Color32::DEBUG_COLOR));

        let node = snarl.get_node_mut(node_idx);

        ui.set_height(16.0 * scale);
        ui.set_width(128.0 * scale);
        ui.with_layout(
            Layout::left_to_right(Align::Min).with_cross_align(Align::Center),
            |ui| {
                ui.add_space(20.0 * scale);
                match node {
                    NoiseNode::Abs(_) => {
                        ui.label("Abs");
                    }
                    NoiseNode::Add(_) => {
                        ui.label("Add");
                    }
                    NoiseNode::BasicMulti(node) => {
                        ui.label("Basic Multi");
                        self.source_ty_combo_box(ui, &mut node.source_ty, node_idx);
                    }
                    NoiseNode::Billow(node) => {
                        ui.label("Billow");
                        self.source_ty_combo_box(ui, &mut node.source_ty, node_idx);
                    }
                    NoiseNode::Blend(_) => {
                        ui.label("Blend");
                    }
                    NoiseNode::Checkerboard(_) => {
                        ui.label("Checkerboard");
                    }
                    NoiseNode::Clamp(_) => {
                        ui.label("Clamp");
                    }
                    NoiseNode::ControlPoint(_) => {
                        ui.label("Control Point");
                    }
                    NoiseNode::Curve(node) => {
                        ui.label("Curve");

                        while let Some(None) = node.control_point_node_indices.last() {
                            node.control_point_node_indices.pop();
                        }
                    }
                    NoiseNode::Cylinders(_) => {
                        ui.label("Cylinders");
                    }
                    NoiseNode::Displace(_) => {
                        ui.label("Displace");
                    }
                    NoiseNode::Exponent(_) => {
                        ui.label("Exponent");
                    }
                    NoiseNode::F64(node) => {
                        ui.label("Decimal");
                        ui.add(TextEdit::singleline(&mut node.name).desired_width(50.0 * scale));

                        if ui
                            .add(
                                DragValue::new(&mut node.value)
                                    .min_decimals(2)
                                    .max_decimals(2)
                                    .speed(0.01),
                            )
                            .changed()
                        {
                            self.updated_node_indices.insert(node_idx);
                        }
                    }
                    NoiseNode::F64Operation(ConstantOpNode { op_ty, .. })
                    | NoiseNode::Operation(ConstantOpNode { op_ty, .. })
                    | NoiseNode::U32Operation(ConstantOpNode { op_ty, .. }) => {
                        ui.label(match op_ty {
                            OpType::Add => "Add",
                            OpType::Divide => "Divide",
                            OpType::Multiply => "Multiply",
                            OpType::Subtract => "Subtract",
                        });
                    }
                    NoiseNode::Fbm(node) => {
                        ui.label("fBm");
                        self.source_ty_combo_box(ui, &mut node.source_ty, node_idx);
                    }
                    NoiseNode::HybridMulti(node) => {
                        ui.label("Hybrid Multi");
                        self.source_ty_combo_box(ui, &mut node.source_ty, node_idx);
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
                    NoiseNode::Negate(_) => {
                        ui.label("Negate");
                    }
                    NoiseNode::OpenSimplex(_) => {
                        ui.label("Open Simplex");
                    }
                    NoiseNode::Perlin(_) => {
                        ui.label("Perlin");
                    }
                    NoiseNode::PerlinSurflet(_) => {
                        ui.label("Perlin Surflet");
                    }
                    NoiseNode::Power(_) => {
                        ui.label("Power");
                    }
                    NoiseNode::RigidMulti(node) => {
                        ui.label("Rigid Multi");
                        self.source_ty_combo_box(ui, &mut node.source_ty, node_idx);
                    }
                    NoiseNode::RotatePoint(_) => {
                        ui.label("Rotate Point");
                    }
                    NoiseNode::ScaleBias(_) => {
                        ui.label("Scale + Bias");
                    }
                    NoiseNode::ScalePoint(_) => {
                        ui.label("Scale Point");
                    }
                    NoiseNode::Select(_) => {
                        ui.label("Select");
                    }
                    NoiseNode::Simplex(_) => {
                        ui.label("Simplex");
                    }
                    NoiseNode::SuperSimplex(_) => {
                        ui.label("Super Simplex");
                    }
                    NoiseNode::Terrace(node) => {
                        ui.label("Terrace");
                        if ui.checkbox(&mut node.inverted, "Inverted").changed() {
                            self.updated_node_indices.insert(node_idx);
                        }

                        while let Some(None) = node.control_point_node_indices.last() {
                            node.control_point_node_indices.pop();
                        }
                    }
                    NoiseNode::TranslatePoint(_) => {
                        ui.label("Translate Point");
                    }
                    NoiseNode::Turbulence(node) => {
                        ui.label("Turbulence");
                        self.source_ty_combo_box(ui, &mut node.source_ty, node_idx);
                    }
                    NoiseNode::U32(node) => {
                        ui.label("Integer");
                        ui.add(TextEdit::singleline(&mut node.name).desired_width(50.0 * scale));

                        if ui.add(DragValue::new(&mut node.value)).changed() {
                            self.updated_node_indices.insert(node_idx);
                        }
                    }
                    NoiseNode::Value(_) => {
                        ui.label("Value");
                    }
                    NoiseNode::Worley(node) => {
                        ui.label("Worley");
                        self.distance_fn_combo_box(ui, &mut node.distance_fn, node_idx);
                        self.return_ty_combo_box(ui, &mut node.return_ty, node_idx);
                    }
                }
            },
        );
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
            NoiseNode::Abs(_)
            | NoiseNode::Checkerboard(_)
            | NoiseNode::Cylinders(_)
            | NoiseNode::OpenSimplex(_)
            | NoiseNode::Perlin(_)
            | NoiseNode::PerlinSurflet(_)
            | NoiseNode::Negate(_)
            | NoiseNode::Simplex(_)
            | NoiseNode::SuperSimplex(_)
            | NoiseNode::Value(_) => 1,
            NoiseNode::Add(_)
            | NoiseNode::ControlPoint(_)
            | NoiseNode::Exponent(_)
            | NoiseNode::F64Operation(_)
            | NoiseNode::Min(_)
            | NoiseNode::Max(_)
            | NoiseNode::Multiply(_)
            | NoiseNode::Operation(_)
            | NoiseNode::Power(_)
            | NoiseNode::U32Operation(_)
            | NoiseNode::Worley(_) => 2,
            NoiseNode::Blend(_) | NoiseNode::Clamp(_) | NoiseNode::ScaleBias(_) => 3,
            NoiseNode::BasicMulti(_)
            | NoiseNode::Billow(_)
            | NoiseNode::Displace(_)
            | NoiseNode::Fbm(_)
            | NoiseNode::HybridMulti(_)
            | NoiseNode::RotatePoint(_)
            | NoiseNode::ScalePoint(_)
            | NoiseNode::TranslatePoint(_)
            | NoiseNode::Turbulence(_) => 5,
            NoiseNode::RigidMulti(_) | NoiseNode::Select(_) => 6,
            NoiseNode::Curve(node) => {
                (node.control_point_node_indices.len()
                    + node.control_point_node_indices.iter().all(Option::is_some) as usize)
                    .max(4)
                    + 1
            }
            NoiseNode::Terrace(node) => {
                (node.control_point_node_indices.len()
                    + node.control_point_node_indices.iter().all(Option::is_some) as usize)
                    .max(2)
                    + 1
            }
        }
    }

    fn outputs(&mut self, _node: &NoiseNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) -> PinInfo {
        // Handle disconnections by resetting node pins to the value of the previous node
        // This happens when you right-click on a wire (snarl does not tell use about that)
        if pin.remotes.is_empty() {
            match (pin.id.input, snarl.get_node(pin.id.node)) {
                (
                    0,
                    &NoiseNode::Abs(UnaryNode {
                        input_node_idx: Some(node_idx),
                        ..
                    })
                    | &NoiseNode::Negate(UnaryNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_unary_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::BasicMulti(FractalNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        seed: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    0,
                    &NoiseNode::Checkerboard(CheckerboardNode {
                        size: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_checkerboard_mut()
                        .unwrap()
                        .size = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    0,
                    &NoiseNode::Clamp(ClampNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_clamp_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::ControlPoint(ControlPointNode {
                        input: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_control_point_mut()
                        .unwrap()
                        .input = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    0,
                    &NoiseNode::Curve(CurveNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_curve_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::Cylinders(CylindersNode {
                        frequency: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_cylinders_mut()
                        .unwrap()
                        .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    0,
                    &NoiseNode::Displace(DisplaceNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_displace_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::Exponent(ExponentNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_exponent_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::OpenSimplex(GeneratorNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Perlin(GeneratorNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::PerlinSurflet(GeneratorNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Simplex(GeneratorNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::SuperSimplex(GeneratorNode {
                        seed: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Value(GeneratorNode {
                        seed: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_generator_mut()
                        .unwrap()
                        .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    0,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        seed: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    0,
                    &NoiseNode::RotatePoint(TransformNode {
                        input_node_idx: Some(node_idx),
                        ..
                    })
                    | &NoiseNode::ScalePoint(TransformNode {
                        input_node_idx: Some(node_idx),
                        ..
                    })
                    | &NoiseNode::TranslatePoint(TransformNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_transform_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::ScaleBias(ScaleBiasNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_scale_bias_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::Turbulence(TurbulenceNode {
                        input_node_idx: Some(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_turbulence_mut()
                        .unwrap()
                        .input_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    0,
                    &NoiseNode::Worley(WorleyNode {
                        seed: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_worley_mut()
                        .unwrap()
                        .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    0 | 1,
                    NoiseNode::Add(node)
                    | NoiseNode::Max(node)
                    | NoiseNode::Min(node)
                    | NoiseNode::Multiply(node)
                    | NoiseNode::Power(node),
                ) if node.input_node_indices[pin.id.input].is_some() => {
                    snarl
                        .get_node_mut(node.input_node_indices[pin.id.input].unwrap())
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_combiner_mut()
                        .unwrap()
                        .input_node_indices[pin.id.input] = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (0 | 1, NoiseNode::Blend(node))
                    if node.input_node_indices[pin.id.input].is_some() =>
                {
                    snarl
                        .get_node_mut(node.input_node_indices[pin.id.input].unwrap())
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_blend_mut()
                        .unwrap()
                        .input_node_indices[pin.id.input] = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (0 | 1, NoiseNode::F64Operation(node))
                    if node.inputs[pin.id.input].is_node_idx() =>
                {
                    let node_idx = node.inputs[pin.id.input].as_node_index().unwrap();
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_const_op_f64_mut()
                        .unwrap()
                        .inputs[pin.id.input] = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);

                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                    NoiseNode::propagate_tuple_from_f64_op(pin.id.node, snarl);
                }
                (0 | 1, NoiseNode::Operation(node)) if node.inputs[pin.id.input].is_node_idx() => {
                    let node_idx = node.inputs[pin.id.input].as_node_index().unwrap();
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_const_op_tuple_mut()
                        .unwrap()
                        .inputs[pin.id.input] = Default::default();
                    self.updated_node_indices.insert(pin.id.node);
                }
                (0 | 1, NoiseNode::U32Operation(node))
                    if node.inputs[pin.id.input].is_node_idx() =>
                {
                    let node_idx = node.inputs[pin.id.input].as_node_index().unwrap();
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_const_op_u32_mut()
                        .unwrap()
                        .inputs[pin.id.input] = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);

                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                    NoiseNode::propagate_tuple_from_u32_op(pin.id.node, snarl);
                }
                (0 | 1, NoiseNode::Select(node))
                    if node.input_node_indices[pin.id.input].is_some() =>
                {
                    snarl
                        .get_node_mut(node.input_node_indices[pin.id.input].unwrap())
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_select_mut()
                        .unwrap()
                        .input_node_indices[pin.id.input] = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    1,
                    &NoiseNode::BasicMulti(FractalNode {
                        octaves: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        octaves: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        octaves: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        octaves: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .octaves = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::Clamp(ClampNode {
                        lower_bound: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_clamp_mut()
                        .unwrap()
                        .lower_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::ControlPoint(ControlPointNode {
                        output: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_control_point_mut()
                        .unwrap()
                        .output = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::Exponent(ExponentNode {
                        exponent: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_exponent_mut()
                        .unwrap()
                        .exponent = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        octaves: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .octaves = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::ScaleBias(ScaleBiasNode {
                        scale: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_scale_bias_mut()
                        .unwrap()
                        .scale = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::Turbulence(TurbulenceNode {
                        seed: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_turbulence_mut()
                        .unwrap()
                        .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    1,
                    &NoiseNode::Worley(WorleyNode {
                        frequency: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_worley_mut()
                        .unwrap()
                        .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (1..=4, NoiseNode::Displace(node)) if node.axes[pin.id.input - 1].is_some() => {
                    snarl
                        .get_node_mut(node.axes[pin.id.input - 1].unwrap())
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_displace_mut()
                        .unwrap()
                        .axes[pin.id.input - 1] = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    1..=4,
                    NoiseNode::RotatePoint(node)
                    | NoiseNode::ScalePoint(node)
                    | NoiseNode::TranslatePoint(node),
                ) if node.axes[pin.id.input - 1].is_node_idx() => {
                    let node_idx = node.axes[pin.id.input - 1].as_node_index().unwrap();
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_transform_mut()
                        .unwrap()
                        .axes[pin.id.input - 1] = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    2,
                    &NoiseNode::BasicMulti(FractalNode {
                        frequency: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        frequency: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        frequency: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        frequency: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (2, NoiseNode::Blend(node)) if node.control_node_idx.is_some() => {
                    snarl
                        .get_node_mut(node.control_node_idx.unwrap())
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_blend_mut()
                        .unwrap()
                        .control_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    2,
                    &NoiseNode::Clamp(ClampNode {
                        upper_bound: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_clamp_mut()
                        .unwrap()
                        .upper_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    2,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        frequency: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    2,
                    &NoiseNode::ScaleBias(ScaleBiasNode {
                        bias: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_scale_bias_mut()
                        .unwrap()
                        .bias = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (2, NoiseNode::Select(node)) if node.control_node_idx.is_some() => {
                    snarl
                        .get_node_mut(node.control_node_idx.unwrap())
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_select_mut()
                        .unwrap()
                        .control_node_idx = None;
                    self.updated_node_indices.insert(pin.id.node);
                }
                (
                    2,
                    &NoiseNode::Turbulence(TurbulenceNode {
                        frequency: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_turbulence_mut()
                        .unwrap()
                        .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    3,
                    &NoiseNode::BasicMulti(FractalNode {
                        lacunarity: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        lacunarity: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        lacunarity: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        lacunarity: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .lacunarity = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    3,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        lacunarity: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .lacunarity = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    3,
                    &NoiseNode::Select(SelectNode {
                        lower_bound: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_select_mut()
                        .unwrap()
                        .lower_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    3,
                    &NoiseNode::Turbulence(TurbulenceNode {
                        power: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_turbulence_mut()
                        .unwrap()
                        .power = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    4,
                    &NoiseNode::BasicMulti(FractalNode {
                        persistence: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Billow(FractalNode {
                        persistence: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::Fbm(FractalNode {
                        persistence: Node(node_idx),
                        ..
                    })
                    | &NoiseNode::HybridMulti(FractalNode {
                        persistence: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_fractal_mut()
                        .unwrap()
                        .persistence = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    4,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        persistence: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .persistence = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    4,
                    &NoiseNode::Select(SelectNode {
                        upper_bound: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_select_mut()
                        .unwrap()
                        .upper_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    4,
                    &NoiseNode::Turbulence(TurbulenceNode {
                        roughness: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_turbulence_mut()
                        .unwrap()
                        .roughness = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
                }
                (
                    5,
                    &NoiseNode::RigidMulti(RigidFractalNode {
                        attenuation: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_rigid_fractal_mut()
                        .unwrap()
                        .attenuation = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (
                    5,
                    &NoiseNode::Select(SelectNode {
                        falloff: Node(node_idx),
                        ..
                    }),
                ) => {
                    snarl
                        .get_node_mut(node_idx)
                        .output_node_indices_mut()
                        .remove(&pin.id.node);
                    snarl
                        .get_node_mut(pin.id.node)
                        .as_select_mut()
                        .unwrap()
                        .falloff = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    self.updated_node_indices.insert(pin.id.node);
                    NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                }
                (control_point_idx, NoiseNode::Curve(node)) if control_point_idx > 0 => {
                    let control_point_idx = control_point_idx - 1;

                    if let Some(node_idx) = node
                        .control_point_node_indices
                        .get(control_point_idx)
                        .copied()
                        .flatten()
                    {
                        snarl
                            .get_node_mut(node_idx)
                            .output_node_indices_mut()
                            .remove(&pin.id.node);
                        snarl
                            .get_node_mut(pin.id.node)
                            .as_curve_mut()
                            .unwrap()
                            .control_point_node_indices[control_point_idx] = None;
                        self.updated_node_indices.insert(pin.id.node);
                    }
                }
                (control_point_idx, NoiseNode::Terrace(node)) if control_point_idx > 0 => {
                    let control_point_idx = control_point_idx - 1;

                    if let Some(node_idx) = node
                        .control_point_node_indices
                        .get(control_point_idx)
                        .copied()
                        .flatten()
                    {
                        snarl
                            .get_node_mut(node_idx)
                            .output_node_indices_mut()
                            .remove(&pin.id.node);
                        snarl
                            .get_node_mut(pin.id.node)
                            .as_terrace_mut()
                            .unwrap()
                            .control_point_node_indices[control_point_idx] = None;
                        self.updated_node_indices.insert(pin.id.node);
                    }
                }
                _ => {}
            }
        }

        ui.set_height(16.0 * scale);
        ui.set_width(128.0 * scale);
        ui.with_layout(
            Layout::left_to_right(Align::Min).with_cross_align(Align::Center),
            |ui| {
                ui.add_space(20.0 * scale);
                match (pin.id.input, snarl.get_node_mut(pin.id.node)) {
                    (
                        0,
                        NoiseNode::Abs(UnaryNode { input_node_idx, .. })
                        | NoiseNode::Clamp(ClampNode { input_node_idx, .. })
                        | NoiseNode::Curve(CurveNode { input_node_idx, .. })
                        | NoiseNode::Displace(DisplaceNode { input_node_idx, .. })
                        | NoiseNode::Exponent(ExponentNode { input_node_idx, .. })
                        | NoiseNode::Negate(UnaryNode { input_node_idx, .. })
                        | NoiseNode::RotatePoint(TransformNode { input_node_idx, .. })
                        | NoiseNode::ScaleBias(ScaleBiasNode { input_node_idx, .. })
                        | NoiseNode::ScalePoint(TransformNode { input_node_idx, .. })
                        | NoiseNode::Terrace(TerraceNode { input_node_idx, .. })
                        | NoiseNode::TranslatePoint(TransformNode { input_node_idx, .. })
                        | NoiseNode::Turbulence(TurbulenceNode { input_node_idx, .. }),
                    ) => {
                        ui.label("Source");

                        #[cfg(debug_assertions)]
                        ui.label(
                            RichText::new(format!("#{:?}", input_node_idx))
                                .color(Color32::DEBUG_COLOR),
                        );

                        Self::image_pin_info(true, input_node_idx.is_some())
                    }
                    (
                        0,
                        NoiseNode::BasicMulti(FractalNode { seed, .. })
                        | NoiseNode::Billow(FractalNode { seed, .. })
                        | NoiseNode::Fbm(FractalNode { seed, .. })
                        | NoiseNode::HybridMulti(FractalNode { seed, .. })
                        | NoiseNode::OpenSimplex(GeneratorNode { seed, .. })
                        | NoiseNode::Perlin(GeneratorNode { seed, .. })
                        | NoiseNode::PerlinSurflet(GeneratorNode { seed, .. })
                        | NoiseNode::RigidMulti(RigidFractalNode { seed, .. })
                        | NoiseNode::Simplex(GeneratorNode { seed, .. })
                        | NoiseNode::SuperSimplex(GeneratorNode { seed, .. })
                        | NoiseNode::Value(GeneratorNode { seed, .. })
                        | NoiseNode::Worley(WorleyNode { seed, .. }),
                    ) => {
                        ui.label("Seed");

                        if let Some(value) = seed.as_value_mut() {
                            self.drag_value_u32(ui, scale, value, pin.id.node);

                            Self::u32_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!("#{:?}", seed.as_node_index().unwrap()))
                                    .color(Color32::DEBUG_COLOR),
                            );

                            Self::u32_pin_info(true, true)
                        }
                    }
                    (0, NoiseNode::Checkerboard(CheckerboardNode { size, .. })) => {
                        ui.label("Size");

                        if let Some(value) = size.as_value_mut() {
                            self.drag_value_u32(ui, scale, value, pin.id.node);

                            Self::u32_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!("#{:?}", size.as_node_index().unwrap()))
                                    .color(Color32::DEBUG_COLOR),
                            );

                            Self::u32_pin_info(true, true)
                        }
                    }
                    (0, NoiseNode::ControlPoint(node)) => {
                        ui.label("Input");

                        if let Some(value) = node.input.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.input.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (0, NoiseNode::Cylinders(node)) => {
                        ui.label("Frequency");

                        if let Some(value) = node.frequency.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.frequency.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (
                        0 | 1,
                        NoiseNode::Add(node)
                        | NoiseNode::Min(node)
                        | NoiseNode::Max(node)
                        | NoiseNode::Multiply(node)
                        | NoiseNode::Power(node),
                    ) => {
                        ui.label("Source");

                        #[cfg(debug_assertions)]
                        ui.label(
                            RichText::new(format!("#{:?}", node.input_node_indices[pin.id.input]))
                                .color(Color32::DEBUG_COLOR),
                        );

                        Self::image_pin_info(true, node.input_node_indices[pin.id.input].is_some())
                    }
                    (
                        0 | 1,
                        NoiseNode::Blend(BlendNode {
                            input_node_indices, ..
                        })
                        | NoiseNode::Select(SelectNode {
                            input_node_indices, ..
                        }),
                    ) => {
                        ui.label("Source");

                        #[cfg(debug_assertions)]
                        ui.label(
                            RichText::new(format!("#{:?}", input_node_indices[pin.id.input]))
                                .color(Color32::DEBUG_COLOR),
                        );

                        Self::image_pin_info(true, input_node_indices[pin.id.input].is_some())
                    }
                    (0 | 1, NoiseNode::F64Operation(node)) => {
                        ui.label("Input");

                        if let Some(value) = node.inputs[pin.id.input].as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.inputs[pin.id.input].as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (0 | 1, NoiseNode::Operation(node)) => {
                        ui.label("Input");

                        if node.inputs[pin.id.input].as_node_index().is_none() {
                            Self::operation_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.inputs[pin.id.input].as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::operation_pin_info(true, true)
                        }
                    }
                    (0 | 1, NoiseNode::U32Operation(node)) => {
                        ui.label("Input");

                        if let Some(value) = node.inputs[pin.id.input].as_value_mut() {
                            self.drag_value_u32(ui, scale, value, pin.id.node);

                            Self::u32_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.inputs[pin.id.input].as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::u32_pin_info(true, true)
                        }
                    }
                    (1, NoiseNode::ControlPoint(node)) => {
                        ui.label("Output");

                        if let Some(value) = node.output.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.output.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
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

                        if let Some(value) = octaves.as_value_mut() {
                            self.drag_value_octaves(ui, scale, value, pin.id.node);

                            Self::u32_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!("#{:?}", octaves.as_node_index().unwrap()))
                                    .color(Color32::DEBUG_COLOR),
                            );

                            Self::u32_pin_info(true, true)
                        }
                    }
                    (1, NoiseNode::Clamp(node)) => {
                        ui.label("Lower Bound");

                        if let Some(value) = node.lower_bound.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.lower_bound.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (1, NoiseNode::Exponent(node)) => {
                        ui.label("Exponent");

                        if let Some(value) = node.exponent.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.exponent.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (1, NoiseNode::Turbulence(node)) => {
                        ui.label("Seed");

                        if let Some(value) = node.seed.as_value_mut() {
                            self.drag_value_u32(ui, scale, value, pin.id.node);

                            Self::u32_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!("#{:?}", node.seed.as_node_index().unwrap()))
                                    .color(Color32::DEBUG_COLOR),
                            );

                            Self::u32_pin_info(true, true)
                        }
                    }
                    (1..=4, NoiseNode::Displace(node)) => {
                        ui.label(Self::AXES[pin.id.input - 1]);

                        if node.axes[pin.id.input - 1].is_none() {
                            Self::image_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.axes[pin.id.input - 1].unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::image_pin_info(true, true)
                        }
                    }
                    (
                        1..=4,
                        NoiseNode::RotatePoint(node)
                        | NoiseNode::ScalePoint(node)
                        | NoiseNode::TranslatePoint(node),
                    ) => {
                        ui.label(Self::AXES[pin.id.input - 1]);

                        if let Some(value) = node.axes[pin.id.input - 1].as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.axes[pin.id.input - 1].as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (1, NoiseNode::ScaleBias(node)) => {
                        ui.label("Scale");

                        if let Some(value) = node.scale.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.scale.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (1, NoiseNode::Worley(node)) => {
                        ui.label("Frequency");

                        if let Some(value) = node.frequency.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.frequency.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
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

                        if let Some(value) = frequency.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!("#{:?}", frequency.as_node_index().unwrap()))
                                    .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (
                        2,
                        NoiseNode::Blend(BlendNode {
                            control_node_idx, ..
                        })
                        | NoiseNode::Select(SelectNode {
                            control_node_idx, ..
                        }),
                    ) => {
                        ui.label("Control");

                        #[cfg(debug_assertions)]
                        ui.label(
                            RichText::new(format!("#{:?}", control_node_idx))
                                .color(Color32::DEBUG_COLOR),
                        );

                        Self::image_pin_info(true, control_node_idx.is_some())
                    }
                    (2, NoiseNode::Clamp(node)) => {
                        ui.label("Upper Bound");

                        if let Some(value) = node.upper_bound.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.upper_bound.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (2, NoiseNode::ScaleBias(node)) => {
                        ui.label("Bias");

                        if let Some(value) = node.bias.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!("#{:?}", node.bias.as_node_index().unwrap()))
                                    .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (2, NoiseNode::Turbulence(node)) => {
                        ui.label("Frequency");

                        if let Some(value) = node.frequency.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.frequency.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
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

                        if let Some(value) = lacunarity.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    lacunarity.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (3, NoiseNode::Select(node)) => {
                        ui.label("Lower Bound");

                        if let Some(value) = node.lower_bound.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.lower_bound.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (3, NoiseNode::Turbulence(node)) => {
                        ui.label("Power");

                        if let Some(value) = node.power.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.power.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
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

                        if let Some(value) = persistence.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    persistence.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (4, NoiseNode::Select(node)) => {
                        ui.label("Upper Bound");

                        if let Some(value) = node.upper_bound.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.upper_bound.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (4, NoiseNode::Turbulence(node)) => {
                        ui.label("Roughness");

                        if let Some(value) = node.roughness.as_value_mut() {
                            self.drag_value_u32(ui, scale, value, pin.id.node);

                            Self::u32_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.roughness.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::u32_pin_info(true, true)
                        }
                    }
                    (5, NoiseNode::RigidMulti(node)) => {
                        ui.label("Attenuation");

                        if let Some(value) = node.attenuation.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.attenuation.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (5, NoiseNode::Select(node)) => {
                        ui.label("Falloff");

                        if let Some(value) = node.falloff.as_value_mut() {
                            self.drag_value_f64(ui, scale, value, pin.id.node);

                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.falloff.as_node_index().unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    (control_point_idx, NoiseNode::Curve(node)) => {
                        ui.label("Control Point");

                        let control_point_idx = control_point_idx - 1;

                        #[cfg(debug_assertions)]
                        ui.label(
                            RichText::new(format!(
                                "#{:?}",
                                node.control_point_node_indices
                                    .get(control_point_idx)
                                    .copied()
                            ))
                            .color(Color32::DEBUG_COLOR),
                        );

                        if node
                            .control_point_node_indices
                            .get(control_point_idx)
                            .copied()
                            .flatten()
                            .is_none()
                        {
                            Self::control_point_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.control_point_node_indices
                                        .get(control_point_idx)
                                        .copied()
                                        .flatten()
                                        .unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::control_point_pin_info(true, true)
                        }
                    }
                    (control_point_idx, NoiseNode::Terrace(node)) => {
                        ui.label("Decimal");

                        let control_point_idx = control_point_idx - 1;

                        #[cfg(debug_assertions)]
                        ui.label(
                            RichText::new(format!(
                                "#{:?}",
                                node.control_point_node_indices
                                    .get(control_point_idx)
                                    .copied()
                            ))
                            .color(Color32::DEBUG_COLOR),
                        );

                        if node
                            .control_point_node_indices
                            .get(control_point_idx)
                            .copied()
                            .flatten()
                            .is_none()
                        {
                            Self::f64_pin_info(true, false)
                        } else {
                            #[cfg(debug_assertions)]
                            ui.label(
                                RichText::new(format!(
                                    "#{:?}",
                                    node.control_point_node_indices
                                        .get(control_point_idx)
                                        .copied()
                                        .flatten()
                                        .unwrap()
                                ))
                                .color(Color32::DEBUG_COLOR),
                            );

                            Self::f64_pin_info(true, true)
                        }
                    }
                    _ => unreachable!(),
                }
            },
        )
        .inner
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) -> PinInfo {
        let node = snarl.get_node(pin.id.node);

        if let Some(texture) = node.image().and_then(|image| image.texture.as_ref()) {
            ui.image((texture.id(), texture.size_vec2() * scale));
        }

        match node {
            NoiseNode::Abs(_)
            | NoiseNode::Add(_)
            | NoiseNode::BasicMulti(_)
            | NoiseNode::Billow(_)
            | NoiseNode::Blend(_)
            | NoiseNode::Checkerboard(_)
            | NoiseNode::Clamp(_)
            | NoiseNode::Curve(_)
            | NoiseNode::Cylinders(_)
            | NoiseNode::Displace(_)
            | NoiseNode::Exponent(_)
            | NoiseNode::Fbm(_)
            | NoiseNode::HybridMulti(_)
            | NoiseNode::Min(_)
            | NoiseNode::Max(_)
            | NoiseNode::Multiply(_)
            | NoiseNode::Negate(_)
            | NoiseNode::OpenSimplex(_)
            | NoiseNode::Perlin(_)
            | NoiseNode::PerlinSurflet(_)
            | NoiseNode::Power(_)
            | NoiseNode::RigidMulti(_)
            | NoiseNode::RotatePoint(_)
            | NoiseNode::ScaleBias(_)
            | NoiseNode::ScalePoint(_)
            | NoiseNode::Select(_)
            | NoiseNode::Simplex(_)
            | NoiseNode::SuperSimplex(_)
            | NoiseNode::Terrace(_)
            | NoiseNode::TranslatePoint(_)
            | NoiseNode::Turbulence(_)
            | NoiseNode::Value(_)
            | NoiseNode::Worley(_) => {
                Self::image_pin_info(false, !node.output_node_indices().is_empty())
            }
            NoiseNode::ControlPoint(_) => {
                Self::control_point_pin_info(false, !node.output_node_indices().is_empty())
            }
            NoiseNode::F64(_) | NoiseNode::F64Operation(_) => {
                Self::f64_pin_info(false, !node.output_node_indices().is_empty())
            }
            NoiseNode::Operation(_) => {
                Self::operation_pin_info(false, !node.output_node_indices().is_empty())
            }
            NoiseNode::U32(_) | NoiseNode::U32Operation(_) => {
                Self::u32_pin_info(false, !node.output_node_indices().is_empty())
            }
        }
    }

    fn input_color(
        &mut self,
        _pin: &InPin,
        _style: &Style,
        _snarl: &mut Snarl<NoiseNode>,
    ) -> Color32 {
        unimplemented!()
    }

    fn output_color(
        &mut self,
        _pin: &OutPin,
        _style: &Style,
        _snarl: &mut Snarl<NoiseNode>,
    ) -> Color32 {
        unimplemented!()
    }

    fn graph_menu(&mut self, pos: Pos2, ui: &mut Ui, _scale: f32, snarl: &mut Snarl<NoiseNode>) {
        ui.label("Add node");

        ui.menu_button("Combiners", |ui| {
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
        });
        ui.menu_button("Generators", |ui| {
            if ui.button("Checkerboard").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Checkerboard(Default::default())));
                ui.close_menu();
            }

            if ui.button("Cylinders").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Cylinders(Default::default())));
                ui.close_menu();
            }

            if ui.button("Open Simplex").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::OpenSimplex(Default::default())));
                ui.close_menu();
            }

            if ui.button("Perlin").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Perlin(Default::default())));
                ui.close_menu();
            }

            if ui.button("Perlin Surflet").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::PerlinSurflet(Default::default())));
                ui.close_menu();
            }

            if ui.button("Simplex").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Simplex(Default::default())));
                ui.close_menu();
            }

            if ui.button("Super Simplex").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::SuperSimplex(Default::default())));
                ui.close_menu();
            }

            if ui.button("Value").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Value(Default::default())));
                ui.close_menu();
            }

            if ui.button("Worley").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Worley(Default::default())));
                ui.close_menu();
            }
        });
        ui.menu_button("Fractals", |ui| {
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

            if ui.button("Billow").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Billow(Default::default())));
                ui.close_menu();
            }

            if ui.button("fBm").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Fbm(Default::default())));
                ui.close_menu();
            }
        });
        ui.menu_button("Modifiers", |ui| {
            if ui.button("Abs").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Abs(Default::default())));
                ui.close_menu();
            }

            if ui.button("Clamp").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Clamp(Default::default())));
                ui.close_menu();
            }

            if ui.button("Curve").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Curve(Default::default())));
                ui.close_menu();
            }

            if ui.button("Exponent").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Exponent(Default::default())));
                ui.close_menu();
            }

            if ui.button("Negate").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Negate(Default::default())));
                ui.close_menu();
            }

            if ui.button("Scale + Bias").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::ScaleBias(Default::default())));
                ui.close_menu();
            }

            if ui.button("Terrace").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Terrace(Default::default())));
                ui.close_menu();
            }
        });
        ui.menu_button("Selectors", |ui| {
            if ui.button("Blend").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Blend(Default::default())));
                ui.close_menu();
            }

            if ui.button("Select").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Select(Default::default())));
                ui.close_menu();
            }
        });
        ui.menu_button("Transformers", |ui| {
            if ui.button("Displace").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Displace(Default::default())));
                ui.close_menu();
            }

            if ui.button("Rotate Point").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::RotatePoint(TransformNode::zero())));
                ui.close_menu();
            }

            if ui.button("Scale Point").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::ScalePoint(TransformNode::one())));
                ui.close_menu();
            }

            if ui.button("Translate Point").clicked() {
                self.updated_node_indices.insert(
                    snarl.insert_node(pos, NoiseNode::TranslatePoint(TransformNode::zero())),
                );
                ui.close_menu();
            }

            if ui.button("Turbulence").clicked() {
                self.updated_node_indices
                    .insert(snarl.insert_node(pos, NoiseNode::Turbulence(Default::default())));
                ui.close_menu();
            }
        });
        ui.menu_button("Constants", |ui| {
            if ui.button("Control Point").clicked() {
                snarl.insert_node(pos, NoiseNode::ControlPoint(Default::default()));
                ui.close_menu();
            }

            if ui.button("Decimal").clicked() {
                snarl.insert_node(pos, NoiseNode::F64(Default::default()));
                ui.close_menu();
            }

            if ui.button("Integer").clicked() {
                snarl.insert_node(pos, NoiseNode::U32(Default::default()));
                ui.close_menu();
            }

            ui.separator();
            ui.label("Operations");

            if ui.button("Add").clicked() {
                snarl.insert_node(
                    pos,
                    NoiseNode::Operation(ConstantOpNode::new(OpType::Add, ())),
                );
                ui.close_menu();
            }

            if ui.button("Divide").clicked() {
                snarl.insert_node(
                    pos,
                    NoiseNode::Operation(ConstantOpNode::new(OpType::Divide, ())),
                );
                ui.close_menu();
            }

            if ui.button("Multiply").clicked() {
                snarl.insert_node(
                    pos,
                    NoiseNode::Operation(ConstantOpNode::new(OpType::Multiply, ())),
                );
                ui.close_menu();
            }

            if ui.button("Subtract").clicked() {
                snarl.insert_node(
                    pos,
                    NoiseNode::Operation(ConstantOpNode::new(OpType::Subtract, ())),
                );
                ui.close_menu();
            }
        });
    }

    fn node_menu(
        &mut self,
        node_idx: usize,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<NoiseNode>,
    ) {
        ui.label("Node menu");

        #[cfg(not(target_arch = "wasm32"))]
        {
            let node = snarl.get_node(node_idx);

            match node {
                NoiseNode::ControlPoint(_)
                | NoiseNode::F64(_)
                | NoiseNode::F64Operation(_)
                | NoiseNode::Operation(_)
                | NoiseNode::U32(_)
                | NoiseNode::U32Operation(_) => (),
                _ => {
                    if ui.button("Export File...").clicked() {
                        if let Some(path) = App::file_dialog().save_file() {
                            App::save_as(path, &node.expr(snarl)).unwrap_or_default();
                        }

                        ui.close_menu();
                    }

                    ui.separator();
                }
            }
        }

        if ui.button("Remove").clicked() {
            self.removed_node_indices.insert(node_idx);

            for remote in inputs.iter().flat_map(|input| input.remotes.iter()) {
                snarl
                    .get_node_mut(remote.node)
                    .output_node_indices_mut()
                    .remove(&node_idx);
            }

            for remote in outputs.iter().flat_map(|output| output.remotes.iter()) {
                self.updated_node_indices.insert(remote.node);
                match (remote.input, snarl.get_node(remote.node)) {
                    (0, NoiseNode::Abs(_) | NoiseNode::Negate(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_unary_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (
                        0,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_fractal_mut()
                            .unwrap()
                            .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (0, NoiseNode::Checkerboard(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_checkerboard_mut()
                            .unwrap()
                            .size = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (0, NoiseNode::Clamp(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_clamp_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::ControlPoint(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_control_point_mut()
                            .unwrap()
                            .input = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (0, NoiseNode::Curve(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_curve_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::Cylinders(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_cylinders_mut()
                            .unwrap()
                            .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (0, NoiseNode::Displace(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_displace_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::Exponent(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_exponent_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (
                        0,
                        NoiseNode::OpenSimplex(_)
                        | NoiseNode::Perlin(_)
                        | NoiseNode::PerlinSurflet(_)
                        | NoiseNode::Simplex(_)
                        | NoiseNode::SuperSimplex(_)
                        | NoiseNode::Value(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_generator_mut()
                            .unwrap()
                            .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (0, NoiseNode::RigidMulti(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_rigid_fractal_mut()
                            .unwrap()
                            .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (
                        0,
                        NoiseNode::RotatePoint(_)
                        | NoiseNode::ScalePoint(_)
                        | NoiseNode::TranslatePoint(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_transform_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::ScaleBias(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_scale_bias_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::Terrace(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_terrace_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::Turbulence(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_turbulence_mut()
                            .unwrap()
                            .input_node_idx = None;
                    }
                    (0, NoiseNode::Worley(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_worley_mut()
                            .unwrap()
                            .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (
                        0 | 1,
                        NoiseNode::Add(_)
                        | NoiseNode::Max(_)
                        | NoiseNode::Min(_)
                        | NoiseNode::Multiply(_)
                        | NoiseNode::Power(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_combiner_mut()
                            .unwrap()
                            .input_node_indices[remote.input] = None;
                    }
                    (0 | 1, NoiseNode::Blend(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_blend_mut()
                            .unwrap()
                            .input_node_indices[remote.input] = None;
                    }
                    (0 | 1, NoiseNode::F64Operation(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_const_op_f64_mut()
                            .unwrap()
                            .inputs[remote.input] = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (0 | 1, NoiseNode::Operation(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_const_op_tuple_mut()
                            .unwrap()
                            .inputs[remote.input] = Default::default();
                    }
                    (0 | 1, NoiseNode::Select(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_select_mut()
                            .unwrap()
                            .input_node_indices[remote.input] = None;
                    }
                    (0 | 1, NoiseNode::U32Operation(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_const_op_u32_mut()
                            .unwrap()
                            .inputs[remote.input] = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (
                        1,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_fractal_mut()
                            .unwrap()
                            .octaves = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (1, NoiseNode::Clamp(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_clamp_mut()
                            .unwrap()
                            .lower_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (1, NoiseNode::ControlPoint(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_control_point_mut()
                            .unwrap()
                            .output = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (1, NoiseNode::Exponent(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_exponent_mut()
                            .unwrap()
                            .exponent = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (1, NoiseNode::RigidMulti(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_rigid_fractal_mut()
                            .unwrap()
                            .octaves = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (1, NoiseNode::ScaleBias(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_scale_bias_mut()
                            .unwrap()
                            .scale = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (1, NoiseNode::Turbulence(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_turbulence_mut()
                            .unwrap()
                            .seed = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (1, NoiseNode::Worley(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_worley_mut()
                            .unwrap()
                            .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (1..=4, NoiseNode::Displace(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_displace_mut()
                            .unwrap()
                            .axes[remote.input - 1] = None;
                    }
                    (
                        1..=4,
                        NoiseNode::RotatePoint(_)
                        | NoiseNode::ScalePoint(_)
                        | NoiseNode::TranslatePoint(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_transform_mut()
                            .unwrap()
                            .axes[remote.input - 1] =
                            Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (
                        2,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_fractal_mut()
                            .unwrap()
                            .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (2, NoiseNode::Blend(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_blend_mut()
                            .unwrap()
                            .control_node_idx = None;
                    }
                    (2, NoiseNode::Clamp(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_clamp_mut()
                            .unwrap()
                            .upper_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (2, NoiseNode::RigidMulti(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_rigid_fractal_mut()
                            .unwrap()
                            .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (2, NoiseNode::ScaleBias(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_scale_bias_mut()
                            .unwrap()
                            .bias = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (2, NoiseNode::Select(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_select_mut()
                            .unwrap()
                            .control_node_idx = None;
                    }
                    (2, NoiseNode::Turbulence(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_turbulence_mut()
                            .unwrap()
                            .frequency = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (
                        3,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_fractal_mut()
                            .unwrap()
                            .lacunarity = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (3, NoiseNode::RigidMulti(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_rigid_fractal_mut()
                            .unwrap()
                            .lacunarity = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (3, NoiseNode::Select(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_select_mut()
                            .unwrap()
                            .lower_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (3, NoiseNode::Turbulence(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_turbulence_mut()
                            .unwrap()
                            .power = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (
                        4,
                        NoiseNode::BasicMulti(_)
                        | NoiseNode::Billow(_)
                        | NoiseNode::Fbm(_)
                        | NoiseNode::HybridMulti(_),
                    ) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_fractal_mut()
                            .unwrap()
                            .persistence = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (4, NoiseNode::RigidMulti(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_rigid_fractal_mut()
                            .unwrap()
                            .persistence = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (4, NoiseNode::Select(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_select_mut()
                            .unwrap()
                            .upper_bound = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (4, NoiseNode::Turbulence(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_turbulence_mut()
                            .unwrap()
                            .roughness = Value(snarl.get_node(node_idx).eval_u32(snarl));
                    }
                    (5, NoiseNode::RigidMulti(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_rigid_fractal_mut()
                            .unwrap()
                            .attenuation = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (5, NoiseNode::Select(_)) => {
                        snarl
                            .get_node_mut(remote.node)
                            .as_select_mut()
                            .unwrap()
                            .falloff = Value(snarl.get_node(node_idx).eval_f64(snarl));
                    }
                    (control_point_idx, NoiseNode::Curve(_)) => {
                        let node = snarl.get_node_mut(remote.node).as_curve_mut().unwrap();
                        node.control_point_node_indices[control_point_idx - 1] = None;

                        while let Some(None) = node.control_point_node_indices.last() {
                            node.control_point_node_indices.pop();
                        }
                    }
                    (control_point_idx, NoiseNode::Terrace(_)) => {
                        let node = snarl.get_node_mut(remote.node).as_terrace_mut().unwrap();
                        node.control_point_node_indices[control_point_idx - 1] = None;

                        while let Some(None) = node.control_point_node_indices.last() {
                            node.control_point_node_indices.pop();
                        }
                    }
                    _ => unreachable!(),
                }
            }

            for node_idx in inputs
                .iter()
                .flat_map(|input| input.remotes.iter().map(|remote| remote.node))
                .chain(
                    outputs
                        .iter()
                        .flat_map(|output| output.remotes.iter().map(|remote| remote.node)),
                )
            {
                NoiseNode::propagate_tuple_from_f64_op(node_idx, snarl);
                NoiseNode::propagate_tuple_from_u32_op(node_idx, snarl);
            }

            snarl.remove_node(node_idx);
            ui.close_menu();
        }
    }
}
