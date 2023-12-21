use {
    super::node::{DistanceFunction, FractalNode, ReturnType, SourceType},
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
    std::cell::RefCell,
};

#[derive(Clone, Debug)]
pub struct BlendExpr {
    pub sources: [Box<Expr>; 2],
    pub control: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct ClampExpr {
    pub source: Box<Expr>,

    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct ControlPointExpr {
    pub input_value: f64,
    pub output_value: f64,
}

#[derive(Clone, Debug)]
pub struct CurveExpr {
    pub source: Box<Expr>,

    pub control_points: Vec<ControlPointExpr>,
}

#[derive(Clone, Debug)]
pub struct DisplaceExpr {
    pub source: Box<Expr>,

    pub axes: [Box<Expr>; 4],
}

#[derive(Clone, Debug)]
pub struct ExponentExpr {
    pub source: Box<Expr>,

    pub exponent: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct FractalExpr {
    pub source_ty: SourceType,
    pub seed: u32,
    pub octaves: u32,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Abs(Box<Expr>),
    Add([Box<Expr>; 2]),
    BasicMulti(FractalExpr),
    Billow(FractalExpr),
    Blend(BlendExpr),
    Checkerboard(u32),
    Clamp(ClampExpr),
    Curve(CurveExpr),
    Cylinders(f64),
    Displace(DisplaceExpr),
    Exponent(ExponentExpr),
    F64(f64),
    Fbm(FractalExpr),
    HybridMulti(FractalExpr),
    Max([Box<Expr>; 2]),
    Min([Box<Expr>; 2]),
    Multiply([Box<Expr>; 2]),
    Negate(Box<Expr>),
    OpenSimplex(u32),
    Perlin(u32),
    PerlinSurflet(u32),
    Power([Box<Expr>; 2]),
    RidgedMulti(RigidFractalExpr),
    RotatePoint(TransformExpr),
    ScaleBias(ScaleBiasExpr),
    ScalePoint(TransformExpr),
    Select(SelectExpr),
    Simplex(u32),
    SuperSimplex(u32),
    Terrace(TerraceExpr),
    TranslatePoint(TransformExpr),
    Turbulence(TurbulenceExpr),
    Value(u32),
    Worley(WorleyExpr),
}

impl Expr {
    fn basic_multi<T>(expr: &FractalExpr) -> Box<BasicMulti<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            BasicMulti::<T>::new(expr.seed)
                .set_octaves(expr.octaves.clamp(1, FractalNode::MAX_OCTAVES) as _)
                .set_frequency(expr.frequency)
                .set_lacunarity(expr.lacunarity)
                .set_persistence(expr.persistence),
        )
    }

    fn billow<T>(expr: &FractalExpr) -> Box<Billow<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            Billow::<T>::new(expr.seed)
                .set_octaves(expr.octaves.clamp(1, FractalNode::MAX_OCTAVES) as _)
                .set_frequency(expr.frequency)
                .set_lacunarity(expr.lacunarity)
                .set_persistence(expr.persistence),
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

            for &ControlPointExpr { input_value, .. } in control_points {
                let input_value = OrderedFloat(input_value);
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
            res = res.add_control_point(control_point.input_value, control_point.output_value);
        }

        Box::new(res)
    }

    fn fbm<T>(expr: &FractalExpr) -> Box<Fbm<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            Fbm::<T>::new(expr.seed)
                .set_octaves(expr.octaves.clamp(1, FractalNode::MAX_OCTAVES) as _)
                .set_frequency(expr.frequency)
                .set_lacunarity(expr.lacunarity)
                .set_persistence(expr.persistence),
        )
    }

    fn hybrid_multi<T>(expr: &FractalExpr) -> Box<HybridMulti<T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            HybridMulti::<T>::new(expr.seed)
                .set_octaves(expr.octaves.clamp(1, FractalNode::MAX_OCTAVES) as _)
                .set_frequency(expr.frequency)
                .set_lacunarity(expr.lacunarity)
                .set_persistence(expr.persistence),
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
            &Self::Checkerboard(size) => Box::new(Checkerboard::new(size as _)),
            Self::Clamp(expr) => Box::new(
                Clamp::new(expr.source.noise())
                    .set_lower_bound(expr.lower_bound.min(expr.upper_bound))
                    .set_upper_bound(expr.lower_bound.max(expr.upper_bound)),
            ),
            Self::Curve(expr) => Self::curve(expr),
            &Self::Cylinders(frequency) => Box::new(Cylinders::new().set_frequency(frequency)),
            Self::Displace(expr) => Box::new(Displace::new(
                expr.source.noise(),
                expr.axes[0].noise(),
                expr.axes[1].noise(),
                expr.axes[2].noise(),
                expr.axes[3].noise(),
            )),
            Self::Exponent(expr) => {
                Box::new(Exponent::new(expr.source.noise()).set_exponent(expr.exponent))
            }
            &Self::F64(value) => Box::new(Constant::new(value)),
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
            &Self::OpenSimplex(seed) => Box::new(OpenSimplex::new(seed)),
            &Self::Perlin(seed) => Box::new(Perlin::new(seed)),
            &Self::PerlinSurflet(seed) => Box::new(PerlinSurflet::new(seed)),
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
                expr.axes[0],
                expr.axes[1],
                expr.axes[2],
                expr.axes[3],
            )),
            Self::ScaleBias(expr) => Box::new(
                ScaleBias::new(expr.source.noise())
                    .set_bias(expr.bias)
                    .set_scale(expr.scale),
            ),
            Self::ScalePoint(expr) => {
                Box::new(ScalePoint::new(expr.source.noise()).set_all_scales(
                    expr.axes[0],
                    expr.axes[1],
                    expr.axes[2],
                    expr.axes[3],
                ))
            }
            Self::Select(expr) => Box::new(
                Select::new(
                    expr.sources[0].noise(),
                    expr.sources[1].noise(),
                    expr.control.noise(),
                )
                .set_bounds(expr.lower_bound, expr.upper_bound)
                .set_falloff(expr.falloff),
            ),
            &Self::Simplex(seed) => Box::new(Simplex::new(seed)),
            &Self::SuperSimplex(seed) => Box::new(SuperSimplex::new(seed)),
            Self::Terrace(expr) => Self::terrace(expr),
            Self::TranslatePoint(expr) => Box::new(
                TranslatePoint::new(expr.source.noise()).set_all_translations(
                    expr.axes[0],
                    expr.axes[1],
                    expr.axes[2],
                    expr.axes[3],
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
            &Self::Value(seed) => Box::new(Value::new(seed)),
            Self::Worley(expr) => Box::new(
                Worley::new(expr.seed)
                    .set_frequency(expr.frequency)
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
            RidgedMulti::<T>::new(expr.seed)
                .set_octaves(expr.octaves.clamp(1, FractalNode::MAX_OCTAVES) as _)
                .set_frequency(expr.frequency)
                .set_lacunarity(expr.lacunarity)
                .set_persistence(expr.persistence)
                .set_attenuation(expr.attenuation),
        )
    }

    fn turbulence<T>(expr: &TurbulenceExpr) -> Box<Turbulence<Box<dyn NoiseFn<f64, 3>>, T>>
    where
        T: Default + Seedable,
    {
        Box::new(
            Turbulence::<Box<dyn NoiseFn<f64, 3>>, T>::new(expr.source.noise())
                .set_seed(expr.seed)
                .set_frequency(expr.frequency)
                .set_power(expr.power)
                .set_roughness(expr.roughness as _),
        )
    }

    fn terrace(expr: &TerraceExpr) -> Box<dyn NoiseFn<f64, 3>> {
        fn invalid_inputs(control_points: &[f64]) -> bool {
            debug_assert!(control_points.len() >= 2);

            let first_input = OrderedFloat(control_points[0]);

            for &input_value in &control_points[1..] {
                let input_value = OrderedFloat(input_value);
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

        for control_point in expr.control_points.iter().copied() {
            res = res.add_control_point(control_point);
        }

        Box::new(res)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RigidFractalExpr {
    pub source_ty: SourceType,
    pub seed: u32,
    pub octaves: u32,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
    pub attenuation: f64,
}

#[derive(Clone, Debug)]
pub struct ScaleBiasExpr {
    pub source: Box<Expr>,

    pub scale: f64,
    pub bias: f64,
}

#[derive(Clone, Debug)]
pub struct SelectExpr {
    pub sources: [Box<Expr>; 2],
    pub control: Box<Expr>,

    pub lower_bound: f64,
    pub upper_bound: f64,
    pub falloff: f64,
}

#[derive(Clone, Debug)]
pub struct TerraceExpr {
    pub source: Box<Expr>,

    pub inverted: bool,
    pub control_points: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct TransformExpr {
    pub source: Box<Expr>,

    pub axes: [f64; 4],
}

#[derive(Clone, Debug)]
pub struct TurbulenceExpr {
    pub source: Box<Expr>,

    pub source_ty: SourceType,
    pub seed: u32,
    pub frequency: f64,
    pub power: f64,
    pub roughness: u32,
}

#[derive(Clone, Debug)]
pub struct WorleyExpr {
    pub seed: u32,
    pub frequency: f64,
    pub distance_fn: DistanceFunction,
    pub return_ty: ReturnType,
}
