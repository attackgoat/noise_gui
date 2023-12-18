use egui_snarl::Snarl;

use crate::expr::Expr;

use {
    egui::TextureHandle,
    serde::{Deserialize, Serialize},
    std::collections::HashSet,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct AbsNode {
    pub input_node_idx: Option<usize>,
    pub output_node_indices: HashSet<usize>,
    pub scale: f64,

    #[serde(skip)]
    pub texture: Option<TextureHandle>,

    #[serde(skip)]
    pub version: usize,

    pub x: f64,
    pub y: f64,
}

impl Default for AbsNode {
    fn default() -> Self {
        Self {
            input_node_idx: None,
            output_node_indices: Default::default(),
            scale: 1.0,
            texture: None,
            version: 0,
            x: 0.0,
            y: 0.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConstantNode<T> {
    pub name: String,
    pub output_node_indices: HashSet<usize>,
    pub value: T,
}

impl Default for ConstantNode<f64> {
    fn default() -> Self {
        Self {
            name: "value".to_owned(),
            output_node_indices: Default::default(),
            value: 0.0,
        }
    }
}

impl Default for ConstantNode<u32> {
    fn default() -> Self {
        Self {
            name: "seed".to_owned(),
            output_node_indices: Default::default(),
            value: 0,
        }
    }
}

pub struct ImageMut<'a> {
    pub scale: &'a mut f64,
    pub texture: &'a mut Option<TextureHandle>,
    pub version: &'a mut usize,
    pub x: &'a mut f64,
    pub y: &'a mut f64,
}

pub struct ImageRef<'a> {
    pub scale: f64,
    pub texture: Option<&'a TextureHandle>,
    pub version: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NodeInput<T> {
    Node(usize),
    Value(T),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NoiseNode {
    Abs(AbsNode),
    F64(ConstantNode<f64>),
    Perlin(PerlinNode),
    U32(ConstantNode<u32>),
}

impl NoiseNode {
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

    pub fn expr(&self, snarl: &Snarl<Self>) -> Expr {
        match self {
            Self::Abs(AbsNode {
                input_node_idx: None,
                ..
            }) => Expr::Abs(Box::new(Expr::F64(0.0))),
            &Self::Abs(AbsNode {
                input_node_idx: Some(input_node_idx),
                ..
            }) => Expr::Abs(Box::new(snarl.get_node(input_node_idx).expr(snarl))),
            &Self::Perlin(PerlinNode {
                seed: NodeInput::Value(seed),
                ..
            }) => Expr::Perlin(seed),
            &Self::Perlin(PerlinNode {
                seed: NodeInput::Node(node),
                ..
            }) => Expr::Perlin(snarl.get_node(node).as_const_u32().unwrap()),
            _ => unimplemented!(),
        }
    }

    pub fn has_image(&self) -> bool {
        match self {
            Self::Abs(_) | Self::Perlin(_) => true,
            Self::F64(_) | Self::U32(_) => false,
        }
    }

    pub fn image(&self) -> Option<ImageRef<'_>> {
        match self {
            Self::Abs(AbsNode {
                scale,
                texture,
                version,
                x,
                y,
                ..
            })
            | Self::Perlin(PerlinNode {
                scale,
                texture,
                version,
                x,
                y,
                ..
            }) => Some(ImageRef {
                scale: *scale,
                texture: texture.as_ref(),
                version: *version,
                x: *x,
                y: *y,
            }),
            _ => None,
        }
    }

    pub fn image_mut(&mut self) -> Option<ImageMut<'_>> {
        match self {
            Self::Abs(AbsNode {
                scale,
                texture,
                version,
                x,
                y,
                ..
            })
            | Self::Perlin(PerlinNode {
                scale,
                texture,
                version,
                x,
                y,
                ..
            }) => Some(ImageMut {
                scale,
                texture,
                version,
                x,
                y,
            }),
            _ => None,
        }
    }

    pub fn output_node_indices(&self) -> &HashSet<usize> {
        match self {
            Self::Abs(AbsNode {
                output_node_indices,
                ..
            })
            | Self::F64(ConstantNode {
                output_node_indices,
                ..
            })
            | Self::Perlin(PerlinNode {
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
            | Self::F64(ConstantNode {
                output_node_indices,
                ..
            })
            | Self::Perlin(PerlinNode {
                output_node_indices,
                ..
            })
            | Self::U32(ConstantNode {
                output_node_indices,
                ..
            }) => output_node_indices,
        }
    }

    pub fn texture_handle(&self) -> Option<&TextureHandle> {
        match self {
            Self::Abs(AbsNode { texture, .. }) | Self::Perlin(PerlinNode { texture, .. }) => {
                texture.as_ref()
            }
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PerlinNode {
    pub output_node_indices: HashSet<usize>,
    pub seed: NodeInput<u32>,
    pub scale: f64,

    #[serde(skip)]
    pub texture: Option<TextureHandle>,

    #[serde(skip)]
    pub version: usize,

    pub x: f64,
    pub y: f64,
}

impl Default for PerlinNode {
    fn default() -> Self {
        Self {
            output_node_indices: Default::default(),
            seed: NodeInput::Value(0),
            scale: 1.0,
            texture: None,
            version: 0,
            x: 0.0,
            y: 0.0,
        }
    }
}
