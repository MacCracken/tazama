use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EffectId(pub Uuid);

impl EffectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EffectId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectKind {
    // Video effects
    ColorGrade {
        brightness: f32,
        contrast: f32,
        saturation: f32,
        temperature: f32,
    },
    Crop {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
    },
    Speed {
        factor: f32,
    },
    Transition {
        kind: TransitionKind,
        duration_frames: u64,
    },

    // Audio effects
    FadeIn {
        duration_frames: u64,
    },
    FadeOut {
        duration_frames: u64,
    },
    Volume {
        gain_db: f32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TransitionKind {
    Cut,
    Dissolve,
    Wipe,
    Fade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: EffectId,
    pub kind: EffectKind,
    pub enabled: bool,
}

impl Effect {
    pub fn new(kind: EffectKind) -> Self {
        Self {
            id: EffectId::new(),
            kind,
            enabled: true,
        }
    }
}
