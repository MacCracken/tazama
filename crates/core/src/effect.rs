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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_new_is_enabled() {
        let e = Effect::new(EffectKind::Speed { factor: 2.0 });
        assert!(e.enabled);
    }

    #[test]
    fn effect_id_default() {
        let id1 = EffectId::default();
        let id2 = EffectId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn effect_kinds_construct() {
        let _ = EffectKind::ColorGrade {
            brightness: 0.1,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
        };
        let _ = EffectKind::Crop {
            left: 0.1,
            top: 0.1,
            right: 0.1,
            bottom: 0.1,
        };
        let _ = EffectKind::Speed { factor: 2.0 };
        let _ = EffectKind::Transition {
            kind: TransitionKind::Dissolve,
            duration_frames: 30,
        };
        let _ = EffectKind::FadeIn {
            duration_frames: 15,
        };
        let _ = EffectKind::FadeOut {
            duration_frames: 15,
        };
        let _ = EffectKind::Volume { gain_db: -3.0 };
    }

    #[test]
    fn effect_serde_round_trip() {
        let effect = Effect::new(EffectKind::ColorGrade {
            brightness: 0.5,
            contrast: 1.2,
            saturation: 0.8,
            temperature: -0.1,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, effect.id);
        assert!(back.enabled);
    }
}
