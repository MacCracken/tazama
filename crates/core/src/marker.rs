use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarkerId(pub Uuid);

impl MarkerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MarkerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    White,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    pub id: MarkerId,
    pub name: String,
    pub frame: u64,
    pub color: MarkerColor,
}

impl Marker {
    pub fn new(name: impl Into<String>, frame: u64, color: MarkerColor) -> Self {
        Self {
            id: MarkerId::new(),
            name: name.into(),
            frame,
            color,
        }
    }
}
