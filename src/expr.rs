use noise::{Abs, Constant, NoiseFn, Perlin};

pub enum Expr {
    Abs(Box<Expr>),
    F64(f64),
    Perlin(u32),
}

impl Expr {
    pub fn noise(&self) -> Box<dyn NoiseFn<f64, 3>> {
        match self {
            Self::Abs(expr) => Box::new(Abs::new(expr.noise())),
            &Self::F64(value) => Box::new(Constant::new(value)),
            &Self::Perlin(seed) => Box::new(Perlin::new(seed)),
        }
    }
}
