use {
    super::node::{FractalNode, Source},
    noise::{
        Abs, Add, BasicMulti, Billow, Constant, Fbm, HybridMulti, Max, Min, MultiFractal, Multiply,
        NoiseFn, OpenSimplex, Perlin, PerlinSurflet, Power, RidgedMulti, Seedable, Simplex, Value,
        Worley,
    },
};

#[derive(Clone, Copy, Debug)]
pub struct FractalExpr {
    pub source: Source,

    pub seed: u32,
    pub octaves: u32,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
}

#[derive(Debug)]
pub enum Expr {
    Abs(Box<Expr>),
    Add([Box<Expr>; 2]),
    BasicMulti(FractalExpr),
    Billow(FractalExpr),
    F64(f64),
    Fbm(FractalExpr),
    HybridMulti(FractalExpr),
    Max([Box<Expr>; 2]),
    Min([Box<Expr>; 2]),
    Multiply([Box<Expr>; 2]),
    Perlin(u32),
    Power([Box<Expr>; 2]),
    RidgedMulti(RigidFractalExpr),
}

impl Expr {
    fn basic_multi<T>(expr: FractalExpr) -> Box<BasicMulti<T>>
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

    fn billow<T>(expr: FractalExpr) -> Box<Billow<T>>
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

    fn fbm<T>(expr: FractalExpr) -> Box<Fbm<T>>
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

    fn hybrid_multi<T>(expr: FractalExpr) -> Box<HybridMulti<T>>
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
            &Self::BasicMulti(expr) => match expr.source {
                Source::OpenSimplex => Self::basic_multi::<OpenSimplex>(expr),
                Source::Perlin => Self::basic_multi::<Perlin>(expr),
                Source::PerlinSurflet => Self::basic_multi::<PerlinSurflet>(expr),
                Source::Simplex => Self::basic_multi::<Simplex>(expr),
                Source::SuperSimplex => Self::basic_multi::<OpenSimplex>(expr),
                Source::Value => Self::basic_multi::<Value>(expr),
                Source::Worley => Self::basic_multi::<Worley>(expr),
            },
            &Self::Billow(expr) => match expr.source {
                Source::OpenSimplex => Self::billow::<OpenSimplex>(expr),
                Source::Perlin => Self::billow::<Perlin>(expr),
                Source::PerlinSurflet => Self::billow::<PerlinSurflet>(expr),
                Source::Simplex => Self::billow::<Simplex>(expr),
                Source::SuperSimplex => Self::billow::<OpenSimplex>(expr),
                Source::Value => Self::billow::<Value>(expr),
                Source::Worley => Self::billow::<Worley>(expr),
            },
            &Self::F64(value) => Box::new(Constant::new(value)),
            &Self::Fbm(expr) => match expr.source {
                Source::OpenSimplex => Self::fbm::<OpenSimplex>(expr),
                Source::Perlin => Self::fbm::<Perlin>(expr),
                Source::PerlinSurflet => Self::fbm::<PerlinSurflet>(expr),
                Source::Simplex => Self::fbm::<Simplex>(expr),
                Source::SuperSimplex => Self::fbm::<OpenSimplex>(expr),
                Source::Value => Self::fbm::<Value>(expr),
                Source::Worley => Self::fbm::<Worley>(expr),
            },
            &Self::HybridMulti(expr) => match expr.source {
                Source::OpenSimplex => Self::hybrid_multi::<OpenSimplex>(expr),
                Source::Perlin => Self::hybrid_multi::<Perlin>(expr),
                Source::PerlinSurflet => Self::hybrid_multi::<PerlinSurflet>(expr),
                Source::Simplex => Self::hybrid_multi::<Simplex>(expr),
                Source::SuperSimplex => Self::hybrid_multi::<OpenSimplex>(expr),
                Source::Value => Self::hybrid_multi::<Value>(expr),
                Source::Worley => Self::hybrid_multi::<Worley>(expr),
            },
            Self::Max([source1, source2]) => Box::new(Max::new(source1.noise(), source2.noise())),
            Self::Min([source1, source2]) => Box::new(Min::new(source1.noise(), source2.noise())),
            Self::Multiply([source1, source2]) => {
                Box::new(Multiply::new(source1.noise(), source2.noise()))
            }
            Self::Power([source1, source2]) => {
                Box::new(Power::new(source1.noise(), source2.noise()))
            }
            &Self::Perlin(seed) => Box::new(Perlin::new(seed)),
            &Self::RidgedMulti(expr) => match expr.source {
                Source::OpenSimplex => Self::rigid_multi::<OpenSimplex>(expr),
                Source::Perlin => Self::rigid_multi::<Perlin>(expr),
                Source::PerlinSurflet => Self::rigid_multi::<PerlinSurflet>(expr),
                Source::Simplex => Self::rigid_multi::<Simplex>(expr),
                Source::SuperSimplex => Self::rigid_multi::<OpenSimplex>(expr),
                Source::Value => Self::rigid_multi::<Value>(expr),
                Source::Worley => Self::rigid_multi::<Worley>(expr),
            },
        }
    }

    fn rigid_multi<T>(expr: RigidFractalExpr) -> Box<RidgedMulti<T>>
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
}

#[derive(Clone, Copy, Debug)]
pub struct RigidFractalExpr {
    pub source: Source,

    pub seed: u32,
    pub octaves: u32,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
    pub attenuation: f64,
}
