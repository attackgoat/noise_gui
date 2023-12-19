use {
    super::expr::{Expr, FractalExpr, RigidFractalExpr},
    egui::TextureHandle,
    egui_snarl::Snarl,
    noise::{BasicMulti as Fractal, Perlin as AnySeedable, RidgedMulti as RigidFractal},
    serde::{Deserialize, Serialize},
    std::collections::HashSet,
};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AbsNode {
    pub image: Image,

    pub input_node_idx: Option<usize>,
    pub output_node_indices: HashSet<usize>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CombinerNode {
    pub image: Image,

    pub input_node_indices: [Option<usize>; 2],
    pub output_node_indices: HashSet<usize>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConstantNode<T> {
    pub name: String,

    pub output_node_indices: HashSet<usize>,

    pub value: T,
}

impl<T> Default for ConstantNode<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            name: "value".to_owned(),
            output_node_indices: Default::default(),
            value: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FractalNode {
    pub image: Image,

    pub output_node_indices: HashSet<usize>,

    pub source: Source,
    pub seed: NodeInput<u32>,
    pub octaves: NodeInput<u32>,
    pub frequency: NodeInput<f64>,
    pub lacunarity: NodeInput<f64>,
    pub persistence: NodeInput<f64>,
}

impl FractalNode {
    pub const MAX_OCTAVES: u32 = Fractal::<AnySeedable>::MAX_OCTAVES as _;
}

impl Default for FractalNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            output_node_indices: Default::default(),
            source: Default::default(),
            seed: NodeInput::Value(Fractal::<AnySeedable>::DEFAULT_SEED),
            octaves: NodeInput::Value(Fractal::<AnySeedable>::DEFAULT_OCTAVES as _),
            frequency: NodeInput::Value(Fractal::<AnySeedable>::DEFAULT_FREQUENCY),
            lacunarity: NodeInput::Value(Fractal::<AnySeedable>::DEFAULT_LACUNARITY),
            persistence: NodeInput::Value(Fractal::<AnySeedable>::DEFAULT_PERSISTENCE),
        }
    }
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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum NodeInput<T> {
    Node(usize),
    Value(T),
}

impl<T> NodeInput<T> {
    // pub fn as_value(&self) -> Option<&T> {
    //     if let Self::Value(value) = self {
    //         Some(value)
    //     } else {
    //         None
    //     }
    // }

    pub fn as_value_mut(&mut self) -> Option<&mut T> {
        if let Self::Value(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl<T> Default for NodeInput<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Value(Default::default())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NoiseNode {
    Abs(AbsNode),
    Add(CombinerNode),
    BasicMulti(FractalNode),
    Billow(FractalNode),
    F64(ConstantNode<f64>),
    Fbm(FractalNode),
    HybridMulti(FractalNode),
    Max(CombinerNode),
    Min(CombinerNode),
    Multiply(CombinerNode),
    Perlin(PerlinNode),
    Power(CombinerNode),
    RigidMulti(RigidFractalNode),
    U32(ConstantNode<u32>),
}

impl NoiseNode {
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

    pub fn as_const_f64(&self) -> Option<f64> {
        if let &Self::F64(ConstantNode { value, .. }) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_const_u32(&self) -> Option<u32> {
        if let &Self::U32(ConstantNode { value, .. }) = self {
            Some(value)
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

    pub fn expr(&self, snarl: &Snarl<Self>) -> Expr {
        match self {
            Self::Abs(AbsNode { input_node_idx, .. }) => Expr::Abs(
                input_node_idx
                    .map(|node_idx| Box::new(snarl.get_node(node_idx).expr(snarl)))
                    .unwrap_or_else(|| Box::new(Expr::F64(0.0))),
            ),
            Self::Add(CombinerNode {
                input_node_indices, ..
            }) => Expr::Add(
                input_node_indices
                    .iter()
                    .map(|node_idx| {
                        node_idx
                            .map(|node_idx| Box::new(snarl.get_node(node_idx).expr(snarl)))
                            .unwrap_or_else(|| Box::new(Expr::F64(0.0)))
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ),
            &Self::BasicMulti(FractalNode {
                source,
                seed,
                octaves,
                frequency,
                lacunarity,
                persistence,
                ..
            }) => Expr::BasicMulti(FractalExpr {
                source,
                seed: match seed {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(seed) => seed,
                },
                octaves: match octaves {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(octaves) => octaves,
                },
                frequency: match frequency {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(frequency) => frequency,
                },
                lacunarity: match lacunarity {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(lacunarity) => lacunarity,
                },
                persistence: match persistence {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(persistence) => persistence,
                },
            }),
            &Self::Billow(FractalNode {
                source,
                seed,
                octaves,
                frequency,
                lacunarity,
                persistence,
                ..
            }) => Expr::Billow(FractalExpr {
                source,
                seed: match seed {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(seed) => seed,
                },
                octaves: match octaves {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(octaves) => octaves,
                },
                frequency: match frequency {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(frequency) => frequency,
                },
                lacunarity: match lacunarity {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(lacunarity) => lacunarity,
                },
                persistence: match persistence {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(persistence) => persistence,
                },
            }),
            &Self::F64(ConstantNode { value, .. }) => Expr::F64(value),
            &Self::Fbm(FractalNode {
                source,
                seed,
                octaves,
                frequency,
                lacunarity,
                persistence,
                ..
            }) => Expr::Fbm(FractalExpr {
                source,
                seed: match seed {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(seed) => seed,
                },
                octaves: match octaves {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(octaves) => octaves,
                },
                frequency: match frequency {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(frequency) => frequency,
                },
                lacunarity: match lacunarity {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(lacunarity) => lacunarity,
                },
                persistence: match persistence {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(persistence) => persistence,
                },
            }),
            &Self::HybridMulti(FractalNode {
                source,
                seed,
                octaves,
                frequency,
                lacunarity,
                persistence,
                ..
            }) => Expr::HybridMulti(FractalExpr {
                source,
                seed: match seed {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(seed) => seed,
                },
                octaves: match octaves {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(octaves) => octaves,
                },
                frequency: match frequency {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(frequency) => frequency,
                },
                lacunarity: match lacunarity {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(lacunarity) => lacunarity,
                },
                persistence: match persistence {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(persistence) => persistence,
                },
            }),

            Self::Max(CombinerNode {
                input_node_indices, ..
            }) => Expr::Max(
                input_node_indices
                    .iter()
                    .map(|node_idx| {
                        node_idx
                            .map(|node_idx| Box::new(snarl.get_node(node_idx).expr(snarl)))
                            .unwrap_or_else(|| Box::new(Expr::F64(0.0)))
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ),

            Self::Min(CombinerNode {
                input_node_indices, ..
            }) => Expr::Min(
                input_node_indices
                    .iter()
                    .map(|node_idx| {
                        node_idx
                            .map(|node_idx| Box::new(snarl.get_node(node_idx).expr(snarl)))
                            .unwrap_or_else(|| Box::new(Expr::F64(0.0)))
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ),

            Self::Multiply(CombinerNode {
                input_node_indices, ..
            }) => Expr::Multiply(
                input_node_indices
                    .iter()
                    .map(|node_idx| {
                        node_idx
                            .map(|node_idx| Box::new(snarl.get_node(node_idx).expr(snarl)))
                            .unwrap_or_else(|| Box::new(Expr::F64(0.0)))
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ),

            Self::Power(CombinerNode {
                input_node_indices, ..
            }) => Expr::Power(
                input_node_indices
                    .iter()
                    .map(|node_idx| {
                        node_idx
                            .map(|node_idx| Box::new(snarl.get_node(node_idx).expr(snarl)))
                            .unwrap_or_else(|| Box::new(Expr::F64(0.0)))
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            ),

            &Self::Perlin(PerlinNode { seed, .. }) => Expr::Perlin(match seed {
                NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                NodeInput::Value(seed) => seed,
            }),
            &Self::RigidMulti(RigidFractalNode {
                source,
                seed,
                octaves,
                frequency,
                lacunarity,
                persistence,
                attenuation,
                ..
            }) => Expr::RidgedMulti(RigidFractalExpr {
                source,
                seed: match seed {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(seed) => seed,
                },
                octaves: match octaves {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_u32().unwrap(),
                    NodeInput::Value(octaves) => octaves,
                },
                frequency: match frequency {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(frequency) => frequency,
                },
                lacunarity: match lacunarity {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(lacunarity) => lacunarity,
                },
                persistence: match persistence {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(persistence) => persistence,
                },
                attenuation: match attenuation {
                    NodeInput::Node(node_idx) => snarl.get_node(node_idx).as_const_f64().unwrap(),
                    NodeInput::Value(attenuation) => attenuation,
                },
            }),
            Self::U32(_) => unimplemented!(),
        }
    }

    pub fn has_image(&self) -> bool {
        self.image().is_some()
    }

    pub fn image(&self) -> Option<&Image> {
        match self {
            Self::Abs(AbsNode { image, .. })
            | Self::Add(CombinerNode { image, .. })
            | Self::BasicMulti(FractalNode { image, .. })
            | Self::Billow(FractalNode { image, .. })
            | Self::Fbm(FractalNode { image, .. })
            | Self::HybridMulti(FractalNode { image, .. })
            | Self::Max(CombinerNode { image, .. })
            | Self::Min(CombinerNode { image, .. })
            | Self::Multiply(CombinerNode { image, .. })
            | Self::Perlin(PerlinNode { image, .. })
            | Self::Power(CombinerNode { image, .. })
            | Self::RigidMulti(RigidFractalNode { image, .. }) => Some(image),
            _ => None,
        }
    }

    pub fn image_mut(&mut self) -> Option<&mut Image> {
        match self {
            Self::Abs(AbsNode { image, .. })
            | Self::Add(CombinerNode { image, .. })
            | Self::BasicMulti(FractalNode { image, .. })
            | Self::Billow(FractalNode { image, .. })
            | Self::Fbm(FractalNode { image, .. })
            | Self::HybridMulti(FractalNode { image, .. })
            | Self::Max(CombinerNode { image, .. })
            | Self::Min(CombinerNode { image, .. })
            | Self::Multiply(CombinerNode { image, .. })
            | Self::Perlin(PerlinNode { image, .. })
            | Self::Power(CombinerNode { image, .. })
            | Self::RigidMulti(RigidFractalNode { image, .. }) => Some(image),
            _ => None,
        }
    }

    pub fn output_node_indices(&self) -> &HashSet<usize> {
        match self {
            Self::Abs(AbsNode {
                output_node_indices,
                ..
            })
            | Self::Add(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::BasicMulti(FractalNode {
                output_node_indices,
                ..
            })
            | Self::Billow(FractalNode {
                output_node_indices,
                ..
            })
            | Self::F64(ConstantNode {
                output_node_indices,
                ..
            })
            | Self::Fbm(FractalNode {
                output_node_indices,
                ..
            })
            | Self::HybridMulti(FractalNode {
                output_node_indices,
                ..
            })
            | Self::Max(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::Min(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::Multiply(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::Perlin(PerlinNode {
                output_node_indices,
                ..
            })
            | Self::Power(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::RigidMulti(RigidFractalNode {
                output_node_indices,
                ..
            })
            | Self::U32(ConstantNode {
                output_node_indices,
                ..
            }) => output_node_indices,
        }
    }

    pub fn output_node_indices_mut(&mut self) -> &mut HashSet<usize> {
        match self {
            Self::Abs(AbsNode {
                output_node_indices,
                ..
            })
            | Self::Add(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::BasicMulti(FractalNode {
                output_node_indices,
                ..
            })
            | Self::Billow(FractalNode {
                output_node_indices,
                ..
            })
            | Self::F64(ConstantNode {
                output_node_indices,
                ..
            })
            | Self::Fbm(FractalNode {
                output_node_indices,
                ..
            })
            | Self::HybridMulti(FractalNode {
                output_node_indices,
                ..
            })
            | Self::Max(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::Min(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::Multiply(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::Perlin(PerlinNode {
                output_node_indices,
                ..
            })
            | Self::Power(CombinerNode {
                output_node_indices,
                ..
            })
            | Self::RigidMulti(RigidFractalNode {
                output_node_indices,
                ..
            })
            | Self::U32(ConstantNode {
                output_node_indices,
                ..
            }) => output_node_indices,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PerlinNode {
    pub image: Image,

    pub output_node_indices: HashSet<usize>,

    pub seed: NodeInput<u32>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RigidFractalNode {
    pub image: Image,

    pub output_node_indices: HashSet<usize>,

    pub source: Source,
    pub seed: NodeInput<u32>,
    pub octaves: NodeInput<u32>,
    pub frequency: NodeInput<f64>,
    pub lacunarity: NodeInput<f64>,
    pub persistence: NodeInput<f64>,
    pub attenuation: NodeInput<f64>,
}

impl Default for RigidFractalNode {
    fn default() -> Self {
        Self {
            image: Default::default(),
            output_node_indices: Default::default(),
            source: Default::default(),
            seed: NodeInput::Value(RigidFractal::<AnySeedable>::DEFAULT_SEED),
            octaves: NodeInput::Value(RigidFractal::<AnySeedable>::DEFAULT_OCTAVE_COUNT as _),
            frequency: NodeInput::Value(RigidFractal::<AnySeedable>::DEFAULT_FREQUENCY),
            lacunarity: NodeInput::Value(RigidFractal::<AnySeedable>::DEFAULT_LACUNARITY),
            persistence: NodeInput::Value(RigidFractal::<AnySeedable>::DEFAULT_PERSISTENCE),
            attenuation: NodeInput::Value(RigidFractal::<AnySeedable>::DEFAULT_ATTENUATION),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Source {
    OpenSimplex,
    Perlin,
    PerlinSurflet,
    Simplex,
    SuperSimplex,
    Value,
    Worley,
}

impl Default for Source {
    fn default() -> Self {
        Self::Perlin
    }
}
