use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::keyframe::KeyframeTrack;

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

    // Audio DSP effects
    Eq {
        low_gain_db: f32,
        mid_gain_db: f32,
        high_gain_db: f32,
    },
    Compressor {
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
    },
    NoiseReduction {
        strength: f32,
    },
    Reverb {
        room_size: f32,
        damping: f32,
        wet: f32,
    },

    // Advanced visual effects
    Lut {
        lut_path: String,
    },
    Transform {
        scale_x: f32,
        scale_y: f32,
        translate_x: f32,
        translate_y: f32,
    },
    Text {
        content: String,
        font_family: String,
        font_size: f32,
        color: [f32; 4],
        x: f32,
        y: f32,
    },

    // Plugin effect
    Plugin {
        plugin_id: String,
        params: std::collections::HashMap<String, f32>,
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
    /// Keyframe tracks for animating effect parameters.
    #[serde(default)]
    pub keyframe_tracks: Vec<KeyframeTrack>,
}

impl Effect {
    pub fn new(kind: EffectKind) -> Self {
        Self {
            id: EffectId::new(),
            kind,
            enabled: true,
            keyframe_tracks: Vec::new(),
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
        let _ = EffectKind::Eq {
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
        };
        let _ = EffectKind::Compressor {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
        };
        let _ = EffectKind::NoiseReduction { strength: 0.5 };
        let _ = EffectKind::Reverb {
            room_size: 0.5,
            damping: 0.5,
            wet: 0.3,
        };
        let _ = EffectKind::Lut {
            lut_path: "test.cube".into(),
        };
        let _ = EffectKind::Transform {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        };
        let _ = EffectKind::Text {
            content: "Hello".into(),
            font_family: "Arial".into(),
            font_size: 48.0,
            color: [1.0, 1.0, 1.0, 1.0],
            x: 100.0,
            y: 100.0,
        };
        let _ = EffectKind::Plugin {
            plugin_id: "invert".into(),
            params: std::collections::HashMap::new(),
        };
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

    #[test]
    fn effect_serde_eq() {
        let effect = Effect::new(EffectKind::Eq {
            low_gain_db: 3.0,
            mid_gain_db: -1.5,
            high_gain_db: 6.0,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, effect.id);
        assert!(matches!(back.kind, EffectKind::Eq { .. }));
        if let EffectKind::Eq {
            low_gain_db,
            mid_gain_db,
            high_gain_db,
        } = &back.kind
        {
            assert!((low_gain_db - 3.0).abs() < 1e-6);
            assert!((mid_gain_db - (-1.5)).abs() < 1e-6);
            assert!((high_gain_db - 6.0).abs() < 1e-6);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_serde_compressor() {
        let effect = Effect::new(EffectKind::Compressor {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, effect.id);
        assert!(matches!(back.kind, EffectKind::Compressor { .. }));
    }

    #[test]
    fn effect_serde_noise_reduction() {
        let effect = Effect::new(EffectKind::NoiseReduction { strength: 0.7 });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, effect.id);
        assert!(matches!(back.kind, EffectKind::NoiseReduction { .. }));
        if let EffectKind::NoiseReduction { strength } = &back.kind {
            assert!((strength - 0.7).abs() < 1e-6);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_serde_reverb() {
        let effect = Effect::new(EffectKind::Reverb {
            room_size: 0.8,
            damping: 0.5,
            wet: 0.3,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, effect.id);
        assert!(matches!(back.kind, EffectKind::Reverb { .. }));
    }

    #[test]
    fn effect_serde_lut() {
        let effect = Effect::new(EffectKind::Lut {
            lut_path: "/path/to/lut.cube".into(),
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.kind, EffectKind::Lut { .. }));
        if let EffectKind::Lut { lut_path } = &back.kind {
            assert_eq!(lut_path, "/path/to/lut.cube");
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_serde_transform() {
        let effect = Effect::new(EffectKind::Transform {
            scale_x: 2.0,
            scale_y: 0.5,
            translate_x: 100.0,
            translate_y: -50.0,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.kind, EffectKind::Transform { .. }));
        if let EffectKind::Transform {
            scale_x,
            scale_y,
            translate_x,
            translate_y,
        } = &back.kind
        {
            assert!((scale_x - 2.0).abs() < 1e-6);
            assert!((scale_y - 0.5).abs() < 1e-6);
            assert!((translate_x - 100.0).abs() < 1e-6);
            assert!((translate_y - (-50.0)).abs() < 1e-6);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_serde_text() {
        let effect = Effect::new(EffectKind::Text {
            content: "Hello World".into(),
            font_family: "Helvetica".into(),
            font_size: 36.0,
            color: [1.0, 0.0, 0.0, 1.0],
            x: 50.0,
            y: 100.0,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.kind, EffectKind::Text { .. }));
        if let EffectKind::Text {
            content,
            font_family,
            font_size,
            color,
            x,
            y,
        } = &back.kind
        {
            assert_eq!(content, "Hello World");
            assert_eq!(font_family, "Helvetica");
            assert!((font_size - 36.0).abs() < 1e-6);
            assert_eq!(color, &[1.0, 0.0, 0.0, 1.0]);
            assert!((x - 50.0).abs() < 1e-6);
            assert!((y - 100.0).abs() < 1e-6);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_serde_plugin() {
        let mut params = std::collections::HashMap::new();
        params.insert("intensity".to_string(), 0.75);
        params.insert("mix".to_string(), 1.0);
        let effect = Effect::new(EffectKind::Plugin {
            plugin_id: "my.plugin.v1".into(),
            params,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.kind, EffectKind::Plugin { .. }));
        if let EffectKind::Plugin { plugin_id, params } = &back.kind {
            assert_eq!(plugin_id, "my.plugin.v1");
            assert_eq!(params.len(), 2);
            assert!((params["intensity"] - 0.75).abs() < 1e-6);
            assert!((params["mix"] - 1.0).abs() < 1e-6);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_with_keyframe_tracks_serde_round_trip() {
        let mut effect = Effect::new(EffectKind::Speed { factor: 1.5 });
        let mut track = crate::keyframe::KeyframeTrack::new("speed");
        track.add_keyframe(crate::keyframe::Keyframe::new(
            0,
            1.0,
            crate::keyframe::Interpolation::Linear,
        ));
        track.add_keyframe(crate::keyframe::Keyframe::new(
            30,
            2.0,
            crate::keyframe::Interpolation::Hold,
        ));
        effect.keyframe_tracks.push(track);

        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, effect.id);
        assert_eq!(back.keyframe_tracks.len(), 1);
        assert_eq!(back.keyframe_tracks[0].keyframes.len(), 2);
        assert_eq!(back.keyframe_tracks[0].parameter, "speed");
    }

    #[test]
    fn effect_kind_plugin_empty_params() {
        let effect = Effect::new(EffectKind::Plugin {
            plugin_id: "empty-plugin".into(),
            params: std::collections::HashMap::new(),
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.kind, EffectKind::Plugin { .. }));
        if let EffectKind::Plugin { plugin_id, params } = &back.kind {
            assert_eq!(plugin_id, "empty-plugin");
            assert!(params.is_empty());
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_kind_text_empty_content() {
        let effect = Effect::new(EffectKind::Text {
            content: "".into(),
            font_family: "Arial".into(),
            font_size: 24.0,
            color: [0.0, 0.0, 0.0, 1.0],
            x: 0.0,
            y: 0.0,
        });
        let json = serde_json::to_string(&effect).unwrap();
        let back: Effect = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.kind, EffectKind::Text { .. }));
        if let EffectKind::Text { content, .. } = &back.kind {
            assert_eq!(content, "");
        } else {
            unreachable!();
        }
    }

    #[test]
    fn effect_keyframe_tracks_default_on_deserialization() {
        // Simulate old format JSON without keyframe_tracks field
        let json = r#"{"id":"00000000-0000-0000-0000-000000000001","kind":{"Speed":{"factor":2.0}},"enabled":true}"#;
        let effect: Effect = serde_json::from_str(json).unwrap();
        assert!(effect.keyframe_tracks.is_empty());
        assert!(effect.enabled);
    }
}
