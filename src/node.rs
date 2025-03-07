use {
    egui::TextureHandle,
    egui_snarl::{InPinId, NodeId, OutPinId, Snarl},
    noise::{
        BasicMulti as Fractal, Cylinders, Perlin as AnySeedable, RidgedMulti as RigidFractal,
        Turbulence, Worley,
    },
    noise_expr::{
        BlendExpr, ClampExpr, ControlPointExpr, CurveExpr, DisplaceExpr, DistanceFunction,
        ExponentExpr, Expr, FractalExpr, OpType, ReturnType, RigidFractalExpr, ScaleBiasExpr,
        SelectExpr, SourceType, TerraceExpr, TransformExpr, TurbulenceExpr, Variable, WorleyExpr,
    },
    serde::{Deserialize, Serialize},
    std::{cell::RefCell, collections::HashSet},
};

fn constant(value: f64) -> Box<Expr> {
    Box::new(Expr::Constant(Variable::Anonymous(value)))
}

fn in_pin_expr(snarl: &Snarl<NoiseNode>, node_id: NodeId, input: usize) -> Option<Box<Expr>> {
    map_in_pin(snarl, node_id, input, |node_id| {
        Box::new(snarl.get_node(node_id).unwrap().expr(node_id, snarl))
    })
}

fn in_pin_expr_or_const(
    snarl: &Snarl<NoiseNode>,
    node_id: NodeId,
    input: usize,
    value: f64,
) -> Box<Expr> {
    in_pin_expr_or_else(snarl, node_id, input, || constant(value))
}

fn in_pin_expr_or_else<F>(
    snarl: &Snarl<NoiseNode>,
    node_id: NodeId,
    input: usize,
    f: F,
) -> Box<Expr>
where
    F: FnOnce() -> Box<Expr>,
{
    in_pin_expr(snarl, node_id, input).unwrap_or_else(f)
}

fn map_in_pin<T, U, F>(snarl: &Snarl<T>, node_id: NodeId, input: usize, f: F) -> Option<U>
where
    F: FnOnce(NodeId) -> U,
{
    let remotes = snarl
        .in_pin(InPinId {
            node: node_id,
            input,
        })
        .remotes;

    debug_assert!(
        remotes.len() <= 1,
        "Input pins may only be connected to zero or one nodes"
    );

    remotes.first().map(|remote| f(remote.node))
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct BlendNode {
    pub image: Image,
}

impl BlendNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> BlendExpr {
        BlendExpr {
            sources: (0..2)
                .map(|input| in_pin_expr_or_const(snarl, node_id, input, 0.0))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            control: in_pin_expr_or_const(snarl, node_id, 2, 0.0),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CheckerboardNode {
    pub image: Image,

    pub size: NodeValue<u32>,
}

impl Default for CheckerboardNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            size: NodeValue::Value(0), // TODO: Checkerboard::DEFAULT_SIZE is private!
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ClampNode {
    pub image: Image,

    pub lower_bound: NodeValue<f64>,
    pub upper_bound: NodeValue<f64>,
}

impl ClampNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> ClampExpr {
        ClampExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            lower_bound: self.lower_bound.var(snarl),
            upper_bound: self.upper_bound.var(snarl),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CombinerNode {
    pub image: Image,
}

impl CombinerNode {
    fn expr(
        &self,
        node_id: NodeId,
        snarl: &Snarl<NoiseNode>,
        default_value: f64,
    ) -> [Box<Expr>; 2] {
        (0..2)
            .map(|input| in_pin_expr_or_const(snarl, node_id, input, default_value))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConstantNode<T> {
    pub name: String,

    pub value: T,
}

impl<T> Default for ConstantNode<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            name: "name".to_owned(),
            value: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConstantOpNode<T> {
    pub inputs: [NodeValue<T>; 2],

    pub op_ty: OpType,
}

impl<T> ConstantOpNode<T> {
    pub fn new(op_ty: OpType, value: T) -> Self
    where
        T: Copy,
    {
        Self {
            inputs: [NodeValue::Value(value); 2],
            op_ty,
        }
    }
}

impl ConstantOpNode<f64> {
    fn var(&self, snarl: &Snarl<NoiseNode>) -> Variable<f64> {
        Variable::Operation(
            self.inputs
                .iter()
                .map(|input| Box::new(input.var(snarl)))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            self.op_ty,
        )
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ControlPointNode {
    pub input: NodeValue<f64>,
    pub output: NodeValue<f64>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CurveNode {
    pub image: Image,

    pub control_point_node_ids: Vec<Option<NodeId>>,
}

impl CurveNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> CurveExpr {
        CurveExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            control_points: self
                .control_point_node_ids
                .iter()
                .copied()
                .filter_map(|node_id| {
                    node_id.map(|node_id| {
                        snarl
                            .get_node(node_id)
                            .and_then(NoiseNode::as_control_point)
                            .map(|control_point| ControlPointExpr {
                                input_value: control_point.input.var(snarl),
                                output_value: control_point.output.var(snarl),
                            })
                            .unwrap()
                    })
                })
                .collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CylindersNode {
    pub image: Image,

    pub frequency: NodeValue<f64>,
}

impl Default for CylindersNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            frequency: NodeValue::Value(Cylinders::DEFAULT_FREQUENCY),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DisplaceNode {
    pub image: Image,
}

impl DisplaceNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> DisplaceExpr {
        DisplaceExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            axes: (1..5)
                .map(|input| in_pin_expr_or_const(snarl, node_id, input, 0.0))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExponentNode {
    pub image: Image,

    pub exponent: NodeValue<f64>,
}

impl ExponentNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> ExponentExpr {
        ExponentExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            exponent: self.exponent.var(snarl),
        }
    }
}

impl Default for ExponentNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            exponent: NodeValue::Value(1.0),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FractalNode {
    pub image: Image,

    pub source_ty: SourceType,
    pub seed: NodeValue<u32>,
    pub octaves: NodeValue<u32>,
    pub frequency: NodeValue<f64>,
    pub lacunarity: NodeValue<f64>,
    pub persistence: NodeValue<f64>,
}

impl FractalNode {
    fn expr(&self, snarl: &Snarl<NoiseNode>) -> FractalExpr {
        FractalExpr {
            source_ty: self.source_ty,
            seed: self.seed.var(snarl),
            octaves: self.octaves.var(snarl),
            frequency: self.frequency.var(snarl),
            lacunarity: self.lacunarity.var(snarl),
            persistence: self.persistence.var(snarl),
        }
    }
}

impl Default for FractalNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            source_ty: Default::default(),
            seed: NodeValue::Value(Fractal::<AnySeedable>::DEFAULT_SEED),
            octaves: NodeValue::Value(Fractal::<AnySeedable>::DEFAULT_OCTAVES as _),
            frequency: NodeValue::Value(Fractal::<AnySeedable>::DEFAULT_FREQUENCY),
            lacunarity: NodeValue::Value(Fractal::<AnySeedable>::DEFAULT_LACUNARITY),
            persistence: NodeValue::Value(Fractal::<AnySeedable>::DEFAULT_PERSISTENCE),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GeneratorNode {
    pub image: Image,

    pub seed: NodeValue<u32>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Image {
    pub scale: f64,

    #[serde(skip)]
    pub texture: Option<TextureHandle>,

    #[serde(skip)]
    pub version: usize,

    pub x: f64,
    pub y: f64,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            scale: 4.0,
            texture: None,
            version: 0,
            x: 0.0,
            y: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeValue<T> {
    Node(NodeId),
    Value(T),
}

impl<T> NodeValue<T> {
    pub fn as_node_id(&self) -> Option<NodeId> {
        if let &Self::Node(node_id) = self {
            Some(node_id)
        } else {
            None
        }
    }

    pub fn as_value_mut(&mut self) -> Option<&mut T> {
        if let Self::Value(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn is_node_id(&self) -> bool {
        self.as_node_id().is_some()
    }
}

impl NodeValue<f64> {
    fn eval(self, snarl: &Snarl<NoiseNode>) -> f64 {
        match self {
            Self::Node(node_id) => snarl.get_node(node_id).unwrap().eval_f64(snarl),
            Self::Value(value) => value,
        }
    }

    fn var(self, snarl: &Snarl<NoiseNode>) -> Variable<f64> {
        match self {
            Self::Node(node_id) => match snarl.get_node(node_id).unwrap() {
                NoiseNode::F64(node) => Variable::Named(node.name.clone(), node.value),
                NoiseNode::F64Operation(node) => node.var(snarl),
                _ => unreachable!(),
            },
            Self::Value(value) => Variable::Anonymous(value),
        }
    }
}

impl NodeValue<u32> {
    fn eval(self, snarl: &Snarl<NoiseNode>) -> u32 {
        match self {
            Self::Node(node_id) => snarl.get_node(node_id).unwrap().eval_u32(snarl),
            Self::Value(value) => value,
        }
    }

    fn var(self, snarl: &Snarl<NoiseNode>) -> Variable<u32> {
        match self {
            Self::Node(node_id) => match snarl.get_node(node_id).unwrap() {
                NoiseNode::U32(node) => Variable::Named(node.name.clone(), node.value),
                NoiseNode::U32Operation(node) => Variable::Operation(
                    node.inputs
                        .iter()
                        .map(|input| Box::new(input.var(snarl)))
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                    node.op_ty,
                ),
                _ => unreachable!(),
            },
            Self::Value(value) => Variable::Anonymous(value),
        }
    }
}

impl<T> Default for NodeValue<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Value(Default::default())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NoiseNode {
    Abs(UnaryNode),
    Add(CombinerNode),
    BasicMulti(FractalNode),
    Billow(FractalNode),
    Blend(BlendNode),
    Clamp(ClampNode),
    Checkerboard(CheckerboardNode),
    ControlPoint(ControlPointNode),
    Curve(CurveNode),
    Cylinders(CylindersNode),
    Displace(DisplaceNode),
    Exponent(ExponentNode),
    F64(ConstantNode<f64>),
    F64Operation(ConstantOpNode<f64>),
    Fbm(FractalNode),
    HybridMulti(FractalNode),
    Max(CombinerNode),
    Min(CombinerNode),
    Multiply(CombinerNode),
    Negate(UnaryNode),
    OpenSimplex(GeneratorNode),
    Operation(ConstantOpNode<()>),
    Perlin(GeneratorNode),
    PerlinSurflet(GeneratorNode),
    Power(CombinerNode),
    RigidMulti(RigidFractalNode),
    RotatePoint(TransformNode),
    ScaleBias(ScaleBiasNode),
    ScalePoint(TransformNode),
    Select(SelectNode),
    Simplex(GeneratorNode),
    SuperSimplex(GeneratorNode),
    Terrace(TerraceNode),
    TranslatePoint(TransformNode),
    Turbulence(TurbulenceNode),
    U32(ConstantNode<u32>),
    U32Operation(ConstantOpNode<u32>),
    Value(GeneratorNode),
    Worley(WorleyNode),
}

impl NoiseNode {
    pub fn as_checkerboard_mut(&mut self) -> Option<&mut CheckerboardNode> {
        if let Self::Checkerboard(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_clamp_mut(&mut self) -> Option<&mut ClampNode> {
        if let Self::Clamp(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_const_op_f64(&self) -> Option<&ConstantOpNode<f64>> {
        if let Self::F64Operation(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_const_op_f64_mut(&mut self) -> Option<&mut ConstantOpNode<f64>> {
        if let Self::F64Operation(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_const_op_tuple(&self) -> Option<&ConstantOpNode<()>> {
        if let Self::Operation(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_const_op_tuple_mut(&mut self) -> Option<&mut ConstantOpNode<()>> {
        if let Self::Operation(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_const_op_u32(&self) -> Option<&ConstantOpNode<u32>> {
        if let Self::U32Operation(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_const_op_u32_mut(&mut self) -> Option<&mut ConstantOpNode<u32>> {
        if let Self::U32Operation(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_control_point(&self) -> Option<&ControlPointNode> {
        if let Self::ControlPoint(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_control_point_mut(&mut self) -> Option<&mut ControlPointNode> {
        if let Self::ControlPoint(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_curve_mut(&mut self) -> Option<&mut CurveNode> {
        if let Self::Curve(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_cylinders_mut(&mut self) -> Option<&mut CylindersNode> {
        if let Self::Cylinders(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_exponent_mut(&mut self) -> Option<&mut ExponentNode> {
        if let Self::Exponent(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_fractal_mut(&mut self) -> Option<&mut FractalNode> {
        if let Self::BasicMulti(node)
        | Self::Billow(node)
        | Self::Fbm(node)
        | Self::HybridMulti(node) = self
        {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_generator_mut(&mut self) -> Option<&mut GeneratorNode> {
        if let Self::OpenSimplex(node)
        | Self::Perlin(node)
        | Self::PerlinSurflet(node)
        | Self::Simplex(node)
        | Self::SuperSimplex(node)
        | Self::Value(node) = self
        {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_rigid_fractal_mut(&mut self) -> Option<&mut RigidFractalNode> {
        if let Self::RigidMulti(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_scale_bias_mut(&mut self) -> Option<&mut ScaleBiasNode> {
        if let Self::ScaleBias(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_select_mut(&mut self) -> Option<&mut SelectNode> {
        if let Self::Select(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_terrace_mut(&mut self) -> Option<&mut TerraceNode> {
        if let Self::Terrace(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_transform_mut(&mut self) -> Option<&mut TransformNode> {
        if let Self::RotatePoint(node) | Self::ScalePoint(node) | Self::TranslatePoint(node) = self
        {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_turbulence_mut(&mut self) -> Option<&mut TurbulenceNode> {
        if let Self::Turbulence(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn as_worley_mut(&mut self) -> Option<&mut WorleyNode> {
        if let Self::Worley(node) = self {
            Some(node)
        } else {
            None
        }
    }

    pub fn eval_f64(&self, snarl: &Snarl<Self>) -> f64 {
        match self {
            Self::F64(node) => node.value,
            Self::F64Operation(node) => {
                let (lhs, rhs) = (node.inputs[0].eval(snarl), node.inputs[1].eval(snarl));
                match node.op_ty {
                    OpType::Add => lhs + rhs,
                    OpType::Divide => {
                        if rhs != 0.0 {
                            lhs / rhs
                        } else {
                            0.0
                        }
                    }
                    OpType::Multiply => lhs * rhs,
                    OpType::Subtract => lhs - rhs,
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn eval_u32(&self, snarl: &Snarl<Self>) -> u32 {
        match self {
            Self::U32(node) => node.value,
            Self::U32Operation(node) => {
                let (lhs, rhs) = (node.inputs[0].eval(snarl), node.inputs[1].eval(snarl));
                match node.op_ty {
                    OpType::Add => lhs.checked_add(rhs),
                    OpType::Divide => lhs.checked_div(rhs),
                    OpType::Multiply => lhs.checked_mul(rhs),
                    OpType::Subtract => lhs.checked_sub(rhs),
                }
                .unwrap_or_default()
            }
            _ => unreachable!(),
        }
    }

    pub fn expr(&self, node_id: NodeId, snarl: &Snarl<Self>) -> Expr {
        match self {
            Self::Abs(node) => Expr::Abs(node.expr(node_id, snarl)),
            Self::Add(node) => Expr::Add(node.expr(node_id, snarl, 0.0)),
            Self::BasicMulti(node) => Expr::BasicMulti(node.expr(snarl)),
            Self::Billow(node) => Expr::Billow(node.expr(snarl)),
            Self::Blend(node) => Expr::Blend(node.expr(node_id, snarl)),
            Self::Checkerboard(node) => Expr::Checkerboard(node.size.var(snarl)),
            Self::Clamp(node) => Expr::Clamp(node.expr(node_id, snarl)),
            Self::Curve(node) => Expr::Curve(node.expr(node_id, snarl)),
            Self::Cylinders(node) => Expr::Cylinders(node.frequency.var(snarl)),
            Self::Displace(node) => Expr::Displace(node.expr(node_id, snarl)),
            Self::Exponent(node) => Expr::Exponent(node.expr(node_id, snarl)),
            Self::F64(node) => Expr::Constant(Variable::Named(node.name.clone(), node.value)),
            Self::F64Operation(node) => Expr::Constant(node.var(snarl)),
            Self::Fbm(node) => Expr::Fbm(node.expr(snarl)),
            Self::HybridMulti(node) => Expr::HybridMulti(node.expr(snarl)),
            Self::Max(node) => Expr::Max(node.expr(node_id, snarl, 1.0)),
            Self::Min(node) => Expr::Min(node.expr(node_id, snarl, -1.0)),
            Self::Multiply(node) => Expr::Multiply(node.expr(node_id, snarl, 1.0)),
            Self::Negate(node) => Expr::Negate(node.expr(node_id, snarl)),
            Self::OpenSimplex(node) => Expr::OpenSimplex(node.seed.var(snarl)),
            Self::Perlin(node) => Expr::Perlin(node.seed.var(snarl)),
            Self::PerlinSurflet(node) => Expr::PerlinSurflet(node.seed.var(snarl)),
            Self::Power(node) => Expr::Power(node.expr(node_id, snarl, 1.0)),
            Self::RigidMulti(node) => Expr::RidgedMulti(node.expr(snarl)),
            Self::RotatePoint(node) => Expr::RotatePoint(node.expr(node_id, snarl)),
            Self::ScaleBias(node) => Expr::ScaleBias(node.expr(node_id, snarl)),
            Self::ScalePoint(node) => Expr::ScalePoint(node.expr(node_id, snarl)),
            Self::Select(node) => Expr::Select(node.expr(node_id, snarl)),
            Self::Simplex(node) => Expr::Simplex(node.seed.var(snarl)),
            Self::SuperSimplex(node) => Expr::SuperSimplex(node.seed.var(snarl)),
            Self::Terrace(node) => Expr::Terrace(node.expr(node_id, snarl)),
            Self::TranslatePoint(node) => Expr::TranslatePoint(node.expr(node_id, snarl)),
            Self::Turbulence(node) => Expr::Turbulence(node.expr(node_id, snarl)),
            Self::Value(node) => Expr::Value(node.seed.var(snarl)),
            Self::Worley(node) => Expr::Worley(node.expr(snarl)),
            Self::ControlPoint(_) | Self::Operation(_) | Self::U32(_) | Self::U32Operation(_) => {
                unreachable!()
            }
        }
    }

    pub fn has_image(&self) -> bool {
        self.image().is_some()
    }

    pub fn image(&self) -> Option<&Image> {
        match self {
            Self::Abs(UnaryNode { image, .. })
            | Self::Add(CombinerNode { image, .. })
            | Self::BasicMulti(FractalNode { image, .. })
            | Self::Billow(FractalNode { image, .. })
            | Self::Blend(BlendNode { image, .. })
            | Self::Checkerboard(CheckerboardNode { image, .. })
            | Self::Clamp(ClampNode { image, .. })
            | Self::Curve(CurveNode { image, .. })
            | Self::Cylinders(CylindersNode { image, .. })
            | Self::Displace(DisplaceNode { image, .. })
            | Self::Exponent(ExponentNode { image, .. })
            | Self::Fbm(FractalNode { image, .. })
            | Self::HybridMulti(FractalNode { image, .. })
            | Self::Max(CombinerNode { image, .. })
            | Self::Min(CombinerNode { image, .. })
            | Self::Multiply(CombinerNode { image, .. })
            | Self::Negate(UnaryNode { image, .. })
            | Self::OpenSimplex(GeneratorNode { image, .. })
            | Self::Perlin(GeneratorNode { image, .. })
            | Self::PerlinSurflet(GeneratorNode { image, .. })
            | Self::Power(CombinerNode { image, .. })
            | Self::RigidMulti(RigidFractalNode { image, .. })
            | Self::RotatePoint(TransformNode { image, .. })
            | Self::ScaleBias(ScaleBiasNode { image, .. })
            | Self::ScalePoint(TransformNode { image, .. })
            | Self::Select(SelectNode { image, .. })
            | Self::Simplex(GeneratorNode { image, .. })
            | Self::SuperSimplex(GeneratorNode { image, .. })
            | Self::Terrace(TerraceNode { image, .. })
            | Self::TranslatePoint(TransformNode { image, .. })
            | Self::Turbulence(TurbulenceNode { image, .. })
            | Self::Value(GeneratorNode { image, .. })
            | Self::Worley(WorleyNode { image, .. }) => Some(image),
            Self::ControlPoint(_)
            | Self::F64(_)
            | Self::F64Operation(_)
            | Self::Operation(_)
            | Self::U32(_)
            | Self::U32Operation(_) => None,
        }
    }

    pub fn image_mut(&mut self) -> Option<&mut Image> {
        match self {
            Self::Abs(UnaryNode { image, .. })
            | Self::Add(CombinerNode { image, .. })
            | Self::BasicMulti(FractalNode { image, .. })
            | Self::Billow(FractalNode { image, .. })
            | Self::Blend(BlendNode { image, .. })
            | Self::Checkerboard(CheckerboardNode { image, .. })
            | Self::Clamp(ClampNode { image, .. })
            | Self::Curve(CurveNode { image, .. })
            | Self::Cylinders(CylindersNode { image, .. })
            | Self::Displace(DisplaceNode { image, .. })
            | Self::Exponent(ExponentNode { image, .. })
            | Self::Fbm(FractalNode { image, .. })
            | Self::HybridMulti(FractalNode { image, .. })
            | Self::Max(CombinerNode { image, .. })
            | Self::Min(CombinerNode { image, .. })
            | Self::Multiply(CombinerNode { image, .. })
            | Self::Negate(UnaryNode { image, .. })
            | Self::OpenSimplex(GeneratorNode { image, .. })
            | Self::Perlin(GeneratorNode { image, .. })
            | Self::PerlinSurflet(GeneratorNode { image, .. })
            | Self::Power(CombinerNode { image, .. })
            | Self::RigidMulti(RigidFractalNode { image, .. })
            | Self::RotatePoint(TransformNode { image, .. })
            | Self::ScaleBias(ScaleBiasNode { image, .. })
            | Self::ScalePoint(TransformNode { image, .. })
            | Self::Select(SelectNode { image, .. })
            | Self::Simplex(GeneratorNode { image, .. })
            | Self::SuperSimplex(GeneratorNode { image, .. })
            | Self::Terrace(TerraceNode { image, .. })
            | Self::TranslatePoint(TransformNode { image, .. })
            | Self::Turbulence(TurbulenceNode { image, .. })
            | Self::Value(GeneratorNode { image, .. })
            | Self::Worley(WorleyNode { image, .. }) => Some(image),
            Self::ControlPoint(_)
            | Self::F64(_)
            | Self::F64Operation(_)
            | Self::Operation(_)
            | Self::U32(_)
            | Self::U32Operation(_) => None,
        }
    }

    pub fn propagate_f64_from_tuple_op(node_id: NodeId, snarl: &mut Snarl<Self>) {
        thread_local! {
            static CHILD_NODE_IDS: RefCell<Option<HashSet<NodeId>>> = RefCell::new(Some(Default::default()));
            static NODE_IDS: RefCell<Option<Vec<NodeId>>> = RefCell::new(Some(Default::default()));
        }

        let mut child_node_ids = CHILD_NODE_IDS.take().unwrap();
        let mut node_ids = NODE_IDS.take().unwrap();
        node_ids.push(node_id);

        while let Some(node_id) = node_ids.pop() {
            if child_node_ids.insert(node_id) {
                node_ids.extend(
                    snarl
                        .out_pin(OutPinId {
                            node: node_id,
                            output: 0,
                        })
                        .remotes
                        .iter()
                        .map(|remote| remote.node),
                );

                if let node @ Self::Operation(_) = snarl.get_node_mut(node_id).unwrap() {
                    let op = node.as_const_op_tuple().unwrap().clone();
                    node_ids.extend(op.inputs.iter().filter_map(|input| input.as_node_id()));

                    *node = NoiseNode::F64Operation(ConstantOpNode {
                        inputs: op
                            .inputs
                            .iter()
                            .copied()
                            .map(|input| {
                                input.as_node_id().map(NodeValue::Node).unwrap_or_default()
                            })
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap(),
                        op_ty: op.op_ty,
                    });
                } else {
                    unreachable!();
                }
            }
        }

        child_node_ids.clear();
        CHILD_NODE_IDS.set(Some(child_node_ids));
        NODE_IDS.set(Some(node_ids));
    }

    pub fn propagate_tuple_from_f64_op(node_id: NodeId, snarl: &mut Snarl<Self>) {
        thread_local! {
            static CHILD_NODE_IDS: RefCell<Option<HashSet<NodeId>>> = RefCell::new(Some(Default::default()));
            static NODE_IDS: RefCell<Option<Vec<NodeId>>> = RefCell::new(Some(Default::default()));
        }

        let mut child_node_ids = CHILD_NODE_IDS.take().unwrap();
        let mut node_ids = NODE_IDS.take().unwrap();
        node_ids.push(node_id);

        while let Some(node_id) = node_ids.pop() {
            if child_node_ids.insert(node_id) {
                if let node @ Self::F64Operation(_) = snarl.get_node(node_id).unwrap() {
                    let op = node.as_const_op_f64().unwrap();
                    node_ids.extend(op.inputs.iter().filter_map(|input| input.as_node_id()));
                    node_ids.extend(
                        snarl
                            .out_pin(OutPinId {
                                node: node_id,
                                output: 0,
                            })
                            .remotes
                            .iter()
                            .map(|remote| remote.node),
                    );
                } else {
                    child_node_ids.clear();
                    CHILD_NODE_IDS.set(Some(child_node_ids));

                    node_ids.clear();
                    NODE_IDS.set(Some(node_ids));

                    return;
                }
            }
        }

        for node_id in child_node_ids.drain() {
            let node = snarl.get_node_mut(node_id).unwrap();
            let op = node.as_const_op_f64().unwrap().clone();

            *node = NoiseNode::Operation(ConstantOpNode {
                inputs: op
                    .inputs
                    .iter()
                    .copied()
                    .map(|input| input.as_node_id().map(NodeValue::Node).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                op_ty: op.op_ty,
            });
        }

        CHILD_NODE_IDS.set(Some(child_node_ids));
        NODE_IDS.set(Some(node_ids));
    }

    pub fn propagate_tuple_from_u32_op(node_id: NodeId, snarl: &mut Snarl<Self>) {
        thread_local! {
            static CHILD_NODE_IDS: RefCell<Option<HashSet<NodeId>>> = RefCell::new(Some(Default::default()));
            static NODE_IDS: RefCell<Option<Vec<NodeId>>> = RefCell::new(Some(Default::default()));
        }

        let mut child_node_ids = CHILD_NODE_IDS.take().unwrap();
        let mut node_ids = NODE_IDS.take().unwrap();
        node_ids.push(node_id);

        while let Some(node_id) = node_ids.pop() {
            if child_node_ids.insert(node_id) {
                if let node @ Self::U32Operation(_) = snarl.get_node(node_id).unwrap() {
                    let op = node.as_const_op_u32().unwrap();
                    node_ids.extend(op.inputs.iter().filter_map(|input| input.as_node_id()));
                    node_ids.extend(
                        snarl
                            .out_pin(OutPinId {
                                node: node_id,
                                output: 0,
                            })
                            .remotes
                            .iter()
                            .map(|remote| remote.node),
                    );
                } else {
                    child_node_ids.clear();
                    CHILD_NODE_IDS.set(Some(child_node_ids));

                    node_ids.clear();
                    NODE_IDS.set(Some(node_ids));

                    return;
                }
            }
        }

        for node_id in child_node_ids.drain() {
            let node = snarl.get_node_mut(node_id).unwrap();
            let op = node.as_const_op_u32().unwrap().clone();

            *node = NoiseNode::Operation(ConstantOpNode {
                inputs: op
                    .inputs
                    .iter()
                    .copied()
                    .map(|input| input.as_node_id().map(NodeValue::Node).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                op_ty: op.op_ty,
            });
        }

        CHILD_NODE_IDS.set(Some(child_node_ids));
        NODE_IDS.set(Some(node_ids));
    }

    pub fn propagate_u32_from_tuple_op(node_id: NodeId, snarl: &mut Snarl<Self>) {
        thread_local! {
            static CHILD_NODE_IDS: RefCell<Option<HashSet<NodeId>>> = RefCell::new(Some(Default::default()));
            static NODE_IDS: RefCell<Option<Vec<NodeId>>> = RefCell::new(Some(Default::default()));
        }

        let mut child_node_ids = CHILD_NODE_IDS.take().unwrap();
        let mut node_ids = NODE_IDS.take().unwrap();
        node_ids.push(node_id);

        while let Some(node_id) = node_ids.pop() {
            if child_node_ids.insert(node_id) {
                node_ids.extend(
                    snarl
                        .out_pin(OutPinId {
                            node: node_id,
                            output: 0,
                        })
                        .remotes
                        .iter()
                        .map(|remote| remote.node),
                );

                if let node @ Self::Operation(_) = snarl.get_node_mut(node_id).unwrap() {
                    let op = node.as_const_op_tuple().unwrap().clone();
                    node_ids.extend(op.inputs.iter().filter_map(|input| input.as_node_id()));

                    *node = NoiseNode::U32Operation(ConstantOpNode {
                        inputs: op
                            .inputs
                            .iter()
                            .copied()
                            .map(|input| {
                                input.as_node_id().map(NodeValue::Node).unwrap_or_default()
                            })
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap(),
                        op_ty: op.op_ty,
                    });
                } else {
                    unreachable!();
                }
            }
        }

        child_node_ids.clear();
        CHILD_NODE_IDS.set(Some(child_node_ids));
        NODE_IDS.set(Some(node_ids));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RigidFractalNode {
    pub image: Image,

    pub source_ty: SourceType,
    pub seed: NodeValue<u32>,
    pub octaves: NodeValue<u32>,
    pub frequency: NodeValue<f64>,
    pub lacunarity: NodeValue<f64>,
    pub persistence: NodeValue<f64>,
    pub attenuation: NodeValue<f64>,
}

impl RigidFractalNode {
    fn expr(&self, snarl: &Snarl<NoiseNode>) -> RigidFractalExpr {
        RigidFractalExpr {
            source_ty: self.source_ty,
            seed: self.seed.var(snarl),
            octaves: self.octaves.var(snarl),
            frequency: self.frequency.var(snarl),
            lacunarity: self.lacunarity.var(snarl),
            persistence: self.persistence.var(snarl),
            attenuation: self.attenuation.var(snarl),
        }
    }
}

impl Default for RigidFractalNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            source_ty: Default::default(),
            seed: NodeValue::Value(RigidFractal::<AnySeedable>::DEFAULT_SEED),
            octaves: NodeValue::Value(RigidFractal::<AnySeedable>::DEFAULT_OCTAVE_COUNT as _),
            frequency: NodeValue::Value(RigidFractal::<AnySeedable>::DEFAULT_FREQUENCY),
            lacunarity: NodeValue::Value(RigidFractal::<AnySeedable>::DEFAULT_LACUNARITY),
            persistence: NodeValue::Value(RigidFractal::<AnySeedable>::DEFAULT_PERSISTENCE),
            attenuation: NodeValue::Value(RigidFractal::<AnySeedable>::DEFAULT_ATTENUATION),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ScaleBiasNode {
    pub image: Image,

    pub scale: NodeValue<f64>,
    pub bias: NodeValue<f64>,
}

impl ScaleBiasNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> ScaleBiasExpr {
        ScaleBiasExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            scale: self.scale.var(snarl),
            bias: self.bias.var(snarl),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SelectNode {
    pub image: Image,

    pub lower_bound: NodeValue<f64>,
    pub upper_bound: NodeValue<f64>,
    pub falloff: NodeValue<f64>,
}

impl SelectNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> SelectExpr {
        SelectExpr {
            sources: (0..2)
                .map(|input| in_pin_expr_or_const(snarl, node_id, input, 0.0))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            control: in_pin_expr_or_const(snarl, node_id, 2, 0.0),
            lower_bound: self.lower_bound.var(snarl),
            upper_bound: self.upper_bound.var(snarl),
            falloff: self.falloff.var(snarl),
        }
    }
}

impl Default for SelectNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            lower_bound: NodeValue::Value(0.0),
            upper_bound: NodeValue::Value(1.0),
            falloff: NodeValue::Value(0.0),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TerraceNode {
    pub image: Image,

    pub inverted: bool,
    pub control_point_node_ids: Vec<Option<NodeId>>,
}

impl TerraceNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> TerraceExpr {
        TerraceExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            inverted: self.inverted,
            control_points: self
                .control_point_node_ids
                .iter()
                .copied()
                .filter_map(|node_id| {
                    node_id.map(|node_id| match snarl.get_node(node_id).unwrap() {
                        NoiseNode::F64(node) => Variable::Named(node.name.clone(), node.value),
                        NoiseNode::F64Operation(node) => node.var(snarl),
                        _ => unreachable!(),
                    })
                })
                .collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TransformNode {
    pub image: Image,

    pub axes: [NodeValue<f64>; 4],
}

impl TransformNode {
    fn new(value: f64) -> Self {
        Self {
            image: Default::default(),
            axes: [NodeValue::Value(value); 4],
        }
    }

    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> TransformExpr {
        TransformExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            axes: self
                .axes
                .iter()
                .map(|axis| axis.var(snarl))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    pub fn one() -> Self {
        Self::new(1.0)
    }

    pub fn zero() -> Self {
        Self::new(0.0)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TurbulenceNode {
    pub image: Image,

    pub source_ty: SourceType,
    pub seed: NodeValue<u32>,
    pub frequency: NodeValue<f64>,
    pub power: NodeValue<f64>,
    pub roughness: NodeValue<u32>,
}

impl TurbulenceNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> TurbulenceExpr {
        TurbulenceExpr {
            source: in_pin_expr_or_const(snarl, node_id, 0, 0.0),
            source_ty: self.source_ty,
            seed: self.seed.var(snarl),
            frequency: self.frequency.var(snarl),
            power: self.power.var(snarl),
            roughness: self.roughness.var(snarl),
        }
    }
}

impl Default for TurbulenceNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            source_ty: Default::default(),
            seed: NodeValue::Value(Turbulence::<AnySeedable, AnySeedable>::DEFAULT_SEED),
            frequency: NodeValue::Value(Turbulence::<AnySeedable, AnySeedable>::DEFAULT_FREQUENCY),
            power: NodeValue::Value(Turbulence::<AnySeedable, AnySeedable>::DEFAULT_POWER),
            roughness: NodeValue::Value(
                Turbulence::<AnySeedable, AnySeedable>::DEFAULT_ROUGHNESS as _,
            ),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct UnaryNode {
    pub image: Image,
}

impl UnaryNode {
    fn expr(&self, node_id: NodeId, snarl: &Snarl<NoiseNode>) -> Box<Expr> {
        in_pin_expr_or_const(snarl, node_id, 0, 0.0)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorleyNode {
    pub image: Image,

    pub seed: NodeValue<u32>,
    pub frequency: NodeValue<f64>,
    pub distance_fn: DistanceFunction,
    pub return_ty: ReturnType,
}

impl WorleyNode {
    fn expr(&self, snarl: &Snarl<NoiseNode>) -> WorleyExpr {
        WorleyExpr {
            seed: self.seed.var(snarl),
            frequency: self.frequency.var(snarl),
            distance_fn: self.distance_fn,
            return_ty: self.return_ty,
        }
    }
}

impl Default for WorleyNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            seed: NodeValue::Value(Worley::DEFAULT_SEED),
            frequency: NodeValue::Value(Worley::DEFAULT_FREQUENCY),
            distance_fn: DistanceFunction::Euclidean,
            return_ty: ReturnType::Value,
        }
    }
}
