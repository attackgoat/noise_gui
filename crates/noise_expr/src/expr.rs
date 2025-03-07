use {
    noise::{
        core::worley::{
            self,
            distance_functions::{chebyshev, euclidean, euclidean_squared, manhattan},
        },
        Abs, Add, BasicMulti, Billow, Blend, Checkerboard, Clamp, Constant, Curve, Cylinders,
        Displace, Exponent, Fbm, HybridMulti, Max, Min, MultiFractal, Multiply, Negate, NoiseFn,
        OpenSimplex, Perlin, PerlinSurflet, Power, RidgedMulti, RotatePoint, ScaleBias, ScalePoint,
        Seedable, Select, Simplex, SuperSimplex, Terrace, TranslatePoint, Turbulence, Value,
        Worley,
    },
    ordered_float::OrderedFloat,
    serde::{Deserialize, Serialize},
    std::cell::RefCell,
};

pub const MAX_FRACTAL_OCTAVES: u32 = BasicMulti::<Perlin>::MAX_OCTAVES as _;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlendExpr {
    pub sources: [Box<Expr>; 2],
    pub control: Box<Expr>,
}

impl BlendExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.sources.iter_mut().for_each(|expr| {
            expr.set_f64(name, value);
        });
        self.control.set_f64(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.sources.iter_mut().for_each(|expr| {
            expr.set_u32(name, value);
        });
        self.control.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClampExpr {
    pub source: Box<Expr>,

    pub lower_bound: Variable<f64>,
    pub upper_bound: Variable<f64>,
}

impl ClampExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.lower_bound.set_if_named(name, value);
        self.lower_bound.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ControlPointExpr {
    pub input_value: Variable<f64>,
    pub output_value: Variable<f64>,
}

impl ControlPointExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.input_value.set_if_named(name, value);
        self.output_value.set_if_named(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CurveExpr {
    pub source: Box<Expr>,

    pub control_points: Vec<ControlPointExpr>,
}

impl CurveExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.control_points
            .iter_mut()
            .for_each(|expr| expr.set_f64(name, value));
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DisplaceExpr {
    pub source: Box<Expr>,

    pub axes: [Box<Expr>; 4],
}

impl DisplaceExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.axes.iter_mut().for_each(|expr| {
            expr.set_f64(name, value);
        });
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
        self.axes.iter_mut().for_each(|expr| {
            expr.set_u32(name, value);
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DistanceFunction {
    Chebyshev,
    Euclidean,
    EuclideanSquared,
    Manhattan,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExponentExpr {
    pub source: Box<Expr>,

    pub exponent: Variable<f64>,
}

impl ExponentExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.exponent.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FractalExpr {
    pub source_ty: SourceType,
    pub seed: Variable<u32>,
    pub octaves: Variable<u32>,
    pub frequency: Variable<f64>,
    pub lacunarity: Variable<f64>,
    pub persistence: Variable<f64>,
}

impl FractalExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.frequency.set_if_named(name, value);
        self.lacunarity.set_if_named(name, value);
        self.persistence.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.seed.set_if_named(name, value);
        self.octaves.set_if_named(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Expr {
    Abs(Box<Expr>),
    Add([Box<Expr>; 2]),
    BasicMulti(FractalExpr),
    Billow(FractalExpr),
    Blend(BlendExpr),
    Checkerboard(Variable<u32>),
    Clamp(ClampExpr),
    Constant(Variable<f64>),
    ConstantU32(Variable<u32>),
    Curve(CurveExpr),
    Cylinders(Variable<f64>),
    Displace(DisplaceExpr),
    Exponent(ExponentExpr),
    Fbm(FractalExpr),
    HybridMulti(FractalExpr),
    Max([Box<Expr>; 2]),
    Min([Box<Expr>; 2]),
    Multiply([Box<Expr>; 2]),
    Negate(Box<Expr>),
    OpenSimplex(Variable<u32>),
    Perlin(Variable<u32>),
    PerlinSurflet(Variable<u32>),
    Power([Box<Expr>; 2]),
    RidgedMulti(RigidFractalExpr),
    RotatePoint(TransformExpr),
    ScaleBias(ScaleBiasExpr),
    ScalePoint(TransformExpr),
    Select(SelectExpr),
    Simplex(Variable<u32>),
    SuperSimplex(Variable<u32>),
    Terrace(TerraceExpr),
    TranslatePoint(TransformExpr),
    Turbulence(TurbulenceExpr),
    Value(Variable<u32>),
    Worley(WorleyExpr),
}

impl Expr {
    fn basic_multi<T>(expr: &FractalExpr) -> Box<BasicMulti<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            BasicMulti::<T>::new(expr.seed.value())
                .set_octaves(expr.octaves.value().clamp(1, MAX_FRACTAL_OCTAVES) as _)
                .set_frequency(expr.frequency.value())
                .set_lacunarity(expr.lacunarity.value())
                .set_persistence(expr.persistence.value()),
        )
    }

    fn billow<T>(expr: &FractalExpr) -> Box<Billow<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            Billow::<T>::new(expr.seed.value())
                .set_octaves(expr.octaves.value().clamp(1, MAX_FRACTAL_OCTAVES) as _)
                .set_frequency(expr.frequency.value())
                .set_lacunarity(expr.lacunarity.value())
                .set_persistence(expr.persistence.value()),
        )
    }

    fn curve(expr: &CurveExpr) -> Box<dyn NoiseFn<f64, 3>> {
        fn invalid_inputs(control_points: &[ControlPointExpr]) -> bool {
            debug_assert!(control_points.len() >= 4);

            type Inputs = Vec<OrderedFloat<f64>>;

            thread_local! {
                static INPUTS: RefCell<Option<Inputs>> = RefCell::new(Some(Vec::with_capacity(3)));
            }

            let mut inputs = INPUTS.take().unwrap();

            for ControlPointExpr { input_value, .. } in control_points {
                let input_value = OrderedFloat(input_value.value());
                if let Err(idx) = inputs.binary_search(&input_value) {
                    if inputs.len() == 3 {
                        inputs.clear();
                        INPUTS.set(Some(inputs));

                        return false;
                    }

                    inputs.insert(idx, input_value);
                }
            }

            inputs.clear();
            INPUTS.set(Some(inputs));

            true
        }

        // Make sure the control points are valid (noise-rs panics!)
        if expr.control_points.len() < 4 || invalid_inputs(&expr.control_points) {
            return Box::new(Constant::new(0.0));
        }

        let mut res = Curve::new(expr.source.noise());

        for control_point in &expr.control_points {
            res = res.add_control_point(
                control_point.input_value.value(),
                control_point.output_value.value(),
            );
        }

        Box::new(res)
    }

    fn fbm<T>(expr: &FractalExpr) -> Box<Fbm<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            Fbm::<T>::new(expr.seed.value())
                .set_octaves(expr.octaves.value().clamp(1, MAX_FRACTAL_OCTAVES) as _)
                .set_frequency(expr.frequency.value())
                .set_lacunarity(expr.lacunarity.value())
                .set_persistence(expr.persistence.value()),
        )
    }

    fn hybrid_multi<T>(expr: &FractalExpr) -> Box<HybridMulti<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            HybridMulti::<T>::new(expr.seed.value())
                .set_octaves(expr.octaves.value().clamp(1, MAX_FRACTAL_OCTAVES) as _)
                .set_frequency(expr.frequency.value())
                .set_lacunarity(expr.lacunarity.value())
                .set_persistence(expr.persistence.value()),
        )
    }

    pub fn noise(&self) -> Box<dyn NoiseFn<f64, 3>> {
        match self {
            Self::Abs(expr) => Box::new(Abs::new(expr.noise())),
            Self::Add([source1, source2]) => Box::new(Add::new(source1.noise(), source2.noise())),
            Self::BasicMulti(expr) => match expr.source_ty {
                SourceType::OpenSimplex => Self::basic_multi::<OpenSimplex>(expr),
                SourceType::Perlin => Self::basic_multi::<Perlin>(expr),
                SourceType::PerlinSurflet => Self::basic_multi::<PerlinSurflet>(expr),
                SourceType::Simplex => Self::basic_multi::<Simplex>(expr),
                SourceType::SuperSimplex => Self::basic_multi::<OpenSimplex>(expr),
                SourceType::Value => Self::basic_multi::<Value>(expr),
                SourceType::Worley => Self::basic_multi::<Worley>(expr),
            },
            Self::Billow(expr) => match expr.source_ty {
                SourceType::OpenSimplex => Self::billow::<OpenSimplex>(expr),
                SourceType::Perlin => Self::billow::<Perlin>(expr),
                SourceType::PerlinSurflet => Self::billow::<PerlinSurflet>(expr),
                SourceType::Simplex => Self::billow::<Simplex>(expr),
                SourceType::SuperSimplex => Self::billow::<OpenSimplex>(expr),
                SourceType::Value => Self::billow::<Value>(expr),
                SourceType::Worley => Self::billow::<Worley>(expr),
            },
            Self::Blend(expr) => Box::new(Blend::new(
                expr.sources[0].noise(),
                expr.sources[1].noise(),
                expr.control.noise(),
            )),
            Self::Checkerboard(size) => Box::new(Checkerboard::new(size.value() as _)),
            Self::Clamp(expr) => Box::new(
                Clamp::new(expr.source.noise())
                    .set_lower_bound(expr.lower_bound.value().min(expr.upper_bound.value()))
                    .set_upper_bound(expr.lower_bound.value().max(expr.upper_bound.value())),
            ),
            Self::Constant(value) => Box::new(Constant::new(value.value())),
            Self::ConstantU32(_) => unreachable!(),
            Self::Curve(expr) => Self::curve(expr),
            Self::Cylinders(frequency) => {
                Box::new(Cylinders::new().set_frequency(frequency.value()))
            }
            Self::Displace(expr) => Box::new(Displace::new(
                expr.source.noise(),
                expr.axes[0].noise(),
                expr.axes[1].noise(),
                expr.axes[2].noise(),
                expr.axes[3].noise(),
            )),
            Self::Exponent(expr) => {
                Box::new(Exponent::new(expr.source.noise()).set_exponent(expr.exponent.value()))
            }
            Self::Fbm(expr) => match expr.source_ty {
                SourceType::OpenSimplex => Self::fbm::<OpenSimplex>(expr),
                SourceType::Perlin => Self::fbm::<Perlin>(expr),
                SourceType::PerlinSurflet => Self::fbm::<PerlinSurflet>(expr),
                SourceType::Simplex => Self::fbm::<Simplex>(expr),
                SourceType::SuperSimplex => Self::fbm::<OpenSimplex>(expr),
                SourceType::Value => Self::fbm::<Value>(expr),
                SourceType::Worley => Self::fbm::<Worley>(expr),
            },
            Self::HybridMulti(expr) => match expr.source_ty {
                SourceType::OpenSimplex => Self::hybrid_multi::<OpenSimplex>(expr),
                SourceType::Perlin => Self::hybrid_multi::<Perlin>(expr),
                SourceType::PerlinSurflet => Self::hybrid_multi::<PerlinSurflet>(expr),
                SourceType::Simplex => Self::hybrid_multi::<Simplex>(expr),
                SourceType::SuperSimplex => Self::hybrid_multi::<OpenSimplex>(expr),
                SourceType::Value => Self::hybrid_multi::<Value>(expr),
                SourceType::Worley => Self::hybrid_multi::<Worley>(expr),
            },
            Self::Max([source1, source2]) => Box::new(Max::new(source1.noise(), source2.noise())),
            Self::Min([source1, source2]) => Box::new(Min::new(source1.noise(), source2.noise())),
            Self::Multiply([source1, source2]) => {
                Box::new(Multiply::new(source1.noise(), source2.noise()))
            }
            Self::Negate(expr) => Box::new(Negate::new(expr.noise())),
            Self::OpenSimplex(seed) => Box::new(OpenSimplex::new(seed.value())),
            Self::Perlin(seed) => Box::new(Perlin::new(seed.value())),
            Self::PerlinSurflet(seed) => Box::new(PerlinSurflet::new(seed.value())),
            Self::Power([source1, source2]) => {
                Box::new(Power::new(source1.noise(), source2.noise()))
            }
            Self::RidgedMulti(expr) => match expr.source_ty {
                SourceType::OpenSimplex => Self::rigid_multi::<OpenSimplex>(expr),
                SourceType::Perlin => Self::rigid_multi::<Perlin>(expr),
                SourceType::PerlinSurflet => Self::rigid_multi::<PerlinSurflet>(expr),
                SourceType::Simplex => Self::rigid_multi::<Simplex>(expr),
                SourceType::SuperSimplex => Self::rigid_multi::<OpenSimplex>(expr),
                SourceType::Value => Self::rigid_multi::<Value>(expr),
                SourceType::Worley => Self::rigid_multi::<Worley>(expr),
            },
            Self::RotatePoint(expr) => Box::new(RotatePoint::new(expr.source.noise()).set_angles(
                expr.axes[0].value(),
                expr.axes[1].value(),
                expr.axes[2].value(),
                expr.axes[3].value(),
            )),
            Self::ScaleBias(expr) => Box::new(
                ScaleBias::new(expr.source.noise())
                    .set_bias(expr.bias.value())
                    .set_scale(expr.scale.value()),
            ),
            Self::ScalePoint(expr) => {
                Box::new(ScalePoint::new(expr.source.noise()).set_all_scales(
                    expr.axes[0].value(),
                    expr.axes[1].value(),
                    expr.axes[2].value(),
                    expr.axes[3].value(),
                ))
            }
            Self::Select(expr) => Box::new(
                Select::new(
                    expr.sources[0].noise(),
                    expr.sources[1].noise(),
                    expr.control.noise(),
                )
                .set_bounds(expr.lower_bound.value(), expr.upper_bound.value())
                .set_falloff(expr.falloff.value()),
            ),
            Self::Simplex(seed) => Box::new(Simplex::new(seed.value())),
            Self::SuperSimplex(seed) => Box::new(SuperSimplex::new(seed.value())),
            Self::Terrace(expr) => Self::terrace(expr),
            Self::TranslatePoint(expr) => Box::new(
                TranslatePoint::new(expr.source.noise()).set_all_translations(
                    expr.axes[0].value(),
                    expr.axes[1].value(),
                    expr.axes[2].value(),
                    expr.axes[3].value(),
                ),
            ),
            Self::Turbulence(expr) => match expr.source_ty {
                SourceType::OpenSimplex => Self::turbulence::<OpenSimplex>(expr),
                SourceType::Perlin => Self::turbulence::<Perlin>(expr),
                SourceType::PerlinSurflet => Self::turbulence::<PerlinSurflet>(expr),
                SourceType::Simplex => Self::turbulence::<Simplex>(expr),
                SourceType::SuperSimplex => Self::turbulence::<OpenSimplex>(expr),
                SourceType::Value => Self::turbulence::<Value>(expr),
                SourceType::Worley => Self::turbulence::<Worley>(expr),
            },
            Self::Value(seed) => Box::new(Value::new(seed.value())),
            Self::Worley(expr) => Box::new(
                Worley::new(expr.seed.value())
                    .set_frequency(expr.frequency.value())
                    .set_distance_function(match expr.distance_fn {
                        DistanceFunction::Chebyshev => chebyshev,
                        DistanceFunction::Euclidean => euclidean,
                        DistanceFunction::EuclideanSquared => euclidean_squared,
                        DistanceFunction::Manhattan => manhattan,
                    })
                    .set_return_type(match expr.return_ty {
                        ReturnType::Distance => worley::ReturnType::Distance,
                        ReturnType::Value => worley::ReturnType::Value,
                    }),
            ),
        }
    }

    fn rigid_multi<T>(expr: &RigidFractalExpr) -> Box<RidgedMulti<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            RidgedMulti::<T>::new(expr.seed.value())
                .set_octaves(expr.octaves.value().clamp(1, MAX_FRACTAL_OCTAVES) as _)
                .set_frequency(expr.frequency.value())
                .set_lacunarity(expr.lacunarity.value())
                .set_persistence(expr.persistence.value())
                .set_attenuation(expr.attenuation.value()),
        )
    }

    #[allow(unused)]
    pub fn set_f64(&mut self, name: &str, value: f64) -> &mut Self {
        match self {
            Self::Abs(expr) | Self::Negate(expr) => {
                expr.set_f64(name, value);
            }
            Self::Add(exprs)
            | Self::Max(exprs)
            | Self::Min(exprs)
            | Self::Multiply(exprs)
            | Self::Power(exprs) => exprs.iter_mut().for_each(|expr| {
                expr.set_f64(name, value);
            }),
            Self::BasicMulti(expr)
            | Self::Billow(expr)
            | Self::Fbm(expr)
            | Self::HybridMulti(expr) => expr.set_f64(name, value),
            Self::Blend(expr) => expr.set_f64(name, value),
            Self::Clamp(expr) => expr.set_f64(name, value),
            Self::Constant(expr) | Self::Cylinders(expr) => expr.set_if_named(name, value),
            Self::Curve(expr) => expr.set_f64(name, value),
            Self::Displace(expr) => expr.set_f64(name, value),
            Self::Exponent(expr) => expr.set_f64(name, value),
            Self::RidgedMulti(expr) => expr.set_f64(name, value),
            Self::RotatePoint(expr) | Self::ScalePoint(expr) | Self::TranslatePoint(expr) => {
                expr.set_f64(name, value)
            }
            Self::ScaleBias(expr) => expr.set_f64(name, value),
            Self::Select(expr) => expr.set_f64(name, value),
            Self::Terrace(expr) => expr.set_f64(name, value),
            Self::Turbulence(expr) => expr.set_f64(name, value),
            Self::Worley(expr) => expr.set_f64(name, value),
            Self::Checkerboard(_)
            | Self::ConstantU32(_)
            | Self::OpenSimplex(_)
            | Self::Perlin(_)
            | Self::PerlinSurflet(_)
            | Self::Simplex(_)
            | Self::SuperSimplex(_)
            | Self::Value(_) => (),
        }

        self
    }

    #[allow(unused)]
    pub fn set_u32(&mut self, name: &str, value: u32) -> &mut Self {
        match self {
            Self::Abs(expr) | Self::Negate(expr) => {
                expr.set_u32(name, value);
            }
            Self::Add(exprs)
            | Self::Max(exprs)
            | Self::Min(exprs)
            | Self::Multiply(exprs)
            | Self::Power(exprs) => exprs.iter_mut().for_each(|expr| {
                expr.set_u32(name, value);
            }),
            Self::BasicMulti(expr)
            | Self::Billow(expr)
            | Self::Fbm(expr)
            | Self::HybridMulti(expr) => expr.set_u32(name, value),
            Self::Blend(expr) => expr.set_u32(name, value),
            Self::Checkerboard(expr)
            | Self::ConstantU32(expr)
            | Self::OpenSimplex(expr)
            | Self::Perlin(expr)
            | Self::PerlinSurflet(expr)
            | Self::Simplex(expr)
            | Self::SuperSimplex(expr)
            | Self::Value(expr) => expr.set_if_named(name, value),
            Self::Clamp(expr) => expr.set_u32(name, value),
            Self::Curve(expr) => expr.set_u32(name, value),
            Self::Displace(expr) => expr.set_u32(name, value),
            Self::Exponent(expr) => expr.set_u32(name, value),
            Self::RidgedMulti(expr) => expr.set_u32(name, value),
            Self::RotatePoint(expr) | Self::ScalePoint(expr) | Self::TranslatePoint(expr) => {
                expr.set_u32(name, value)
            }
            Self::Select(expr) => expr.set_u32(name, value),
            Self::ScaleBias(expr) => expr.set_u32(name, value),
            Self::Terrace(expr) => expr.set_u32(name, value),
            Self::Turbulence(expr) => expr.set_u32(name, value),
            Self::Worley(expr) => expr.set_u32(name, value),
            Self::Constant(_) | Self::Cylinders(_) => (),
        }

        self
    }

    fn turbulence<T>(expr: &TurbulenceExpr) -> Box<Turbulence<Box<dyn NoiseFn<f64, 3>>, T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            Turbulence::<Box<dyn NoiseFn<f64, 3>>, T>::new(expr.source.noise())
                .set_seed(expr.seed.value())
                .set_frequency(expr.frequency.value())
                .set_power(expr.power.value())
                .set_roughness(expr.roughness.value() as _),
        )
    }

    fn terrace(expr: &TerraceExpr) -> Box<dyn NoiseFn<f64, 3>> {
        fn invalid_inputs(control_points: &[Variable<f64>]) -> bool {
            debug_assert!(control_points.len() >= 2);

            let first_input = OrderedFloat(control_points[0].value());

            for input_value in &control_points[1..] {
                let input_value = OrderedFloat(input_value.value());
                if input_value != first_input {
                    return false;
                }
            }

            true
        }

        // Make sure the control points are valid (noise-rs panics!)
        if expr.control_points.len() < 2 || invalid_inputs(&expr.control_points) {
            return Box::new(Constant::new(0.0));
        }

        let mut res = Terrace::new(expr.source.noise()).invert_terraces(expr.inverted);

        for control_point in expr.control_points.iter() {
            res = res.add_control_point(control_point.value());
        }

        Box::new(res)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum OpType {
    Add,
    Divide,
    Multiply,
    Subtract,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReturnType {
    Distance,
    Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RigidFractalExpr {
    pub source_ty: SourceType,
    pub seed: Variable<u32>,
    pub octaves: Variable<u32>,
    pub frequency: Variable<f64>,
    pub lacunarity: Variable<f64>,
    pub persistence: Variable<f64>,
    pub attenuation: Variable<f64>,
}

impl RigidFractalExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.frequency.set_if_named(name, value);
        self.lacunarity.set_if_named(name, value);
        self.persistence.set_if_named(name, value);
        self.attenuation.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.seed.set_if_named(name, value);
        self.octaves.set_if_named(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScaleBiasExpr {
    pub source: Box<Expr>,

    pub scale: Variable<f64>,
    pub bias: Variable<f64>,
}

impl ScaleBiasExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.scale.set_if_named(name, value);
        self.bias.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelectExpr {
    pub sources: [Box<Expr>; 2],
    pub control: Box<Expr>,

    pub lower_bound: Variable<f64>,
    pub upper_bound: Variable<f64>,
    pub falloff: Variable<f64>,
}

impl SelectExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.sources.iter_mut().for_each(|expr| {
            expr.set_f64(name, value);
        });
        self.control.set_f64(name, value);
        self.lower_bound.set_if_named(name, value);
        self.upper_bound.set_if_named(name, value);
        self.falloff.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.sources.iter_mut().for_each(|expr| {
            expr.set_u32(name, value);
        });
        self.control.set_u32(name, value);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum SourceType {
    OpenSimplex,
    #[default]
    Perlin,
    PerlinSurflet,
    Simplex,
    SuperSimplex,
    Value,
    Worley,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TerraceExpr {
    pub source: Box<Expr>,

    pub inverted: bool,
    pub control_points: Vec<Variable<f64>>,
}

impl TerraceExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.control_points
            .iter_mut()
            .for_each(|control_point| control_point.set_if_named(name, value));
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransformExpr {
    pub source: Box<Expr>,

    pub axes: [Variable<f64>; 4],
}

impl TransformExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.axes
            .iter_mut()
            .for_each(|axis| axis.set_if_named(name, value));
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TurbulenceExpr {
    pub source: Box<Expr>,

    pub source_ty: SourceType,
    pub seed: Variable<u32>,
    pub frequency: Variable<f64>,
    pub power: Variable<f64>,
    pub roughness: Variable<u32>,
}

impl TurbulenceExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.source.set_f64(name, value);
        self.frequency.set_if_named(name, value);
        self.power.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.source.set_u32(name, value);
        self.seed.set_if_named(name, value);
        self.roughness.set_if_named(name, value);
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Variable<T> {
    #[serde(rename = "Value")]
    Anonymous(T),

    #[serde(rename = "Variable")]
    Named(String, T),

    Operation([Box<Self>; 2], OpType),
}

impl<T> Variable<T> {
    fn set_if_named(&mut self, name: &str, value: T)
    where
        T: Copy,
    {
        match self {
            Self::Anonymous(_) => (),
            Self::Named(named, valued) => {
                if named == name {
                    *valued = value;
                }
            }
            Self::Operation(vars, _) => {
                vars.iter_mut()
                    .for_each(|var| var.set_if_named(name, value));
            }
        }
    }
}

impl Variable<f64> {
    fn value(&self) -> f64 {
        match self {
            Self::Anonymous(value) | Self::Named(_, value) => *value,
            Self::Operation(vars, op) => {
                let (lhs, rhs) = (vars[0].value(), vars[1].value());
                match op {
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
        }
    }
}

impl Variable<u32> {
    fn value(&self) -> u32 {
        match self {
            Self::Anonymous(value) | Self::Named(_, value) => *value,
            Self::Operation(vars, op) => {
                let (lhs, rhs) = (vars[0].value(), vars[1].value());
                match op {
                    OpType::Add => lhs.checked_add(rhs),
                    OpType::Divide => lhs.checked_div(rhs),
                    OpType::Multiply => lhs.checked_mul(rhs),
                    OpType::Subtract => lhs.checked_sub(rhs),
                }
                .unwrap_or_default()
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorleyExpr {
    pub seed: Variable<u32>,
    pub frequency: Variable<f64>,
    pub distance_fn: DistanceFunction,
    pub return_ty: ReturnType,
}

impl WorleyExpr {
    fn set_f64(&mut self, name: &str, value: f64) {
        self.frequency.set_if_named(name, value);
    }

    fn set_u32(&mut self, name: &str, value: u32) {
        self.seed.set_if_named(name, value);
    }
}
