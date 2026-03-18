use serde::{Deserialize, Serialize};

/// Manifest describing a WASM plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub effects: Vec<PluginEffectDef>,
}

/// Definition of an effect provided by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEffectDef {
    pub id: String,
    pub name: String,
    pub params: Vec<PluginParamDef>,
}

/// Definition of a parameter for a plugin effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParamDef {
    pub name: String,
    pub default_value: f32,
    pub min_value: f32,
    pub max_value: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_manifest_serde_round_trip() {
        let manifest = PluginManifest {
            id: "com.example.blur".into(),
            name: "Gaussian Blur".into(),
            version: "1.0.0".into(),
            description: "A simple gaussian blur effect".into(),
            effects: vec![PluginEffectDef {
                id: "blur".into(),
                name: "Blur".into(),
                params: vec![PluginParamDef {
                    name: "radius".into(),
                    default_value: 5.0,
                    min_value: 0.0,
                    max_value: 100.0,
                }],
            }],
        };

        let json = serde_json::to_string(&manifest).expect("serialize");
        let deserialized: PluginManifest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.id, manifest.id);
        assert_eq!(deserialized.name, manifest.name);
        assert_eq!(deserialized.version, manifest.version);
        assert_eq!(deserialized.description, manifest.description);
        assert_eq!(deserialized.effects.len(), 1);
        assert_eq!(deserialized.effects[0].id, "blur");
        assert_eq!(deserialized.effects[0].params.len(), 1);
        assert_eq!(deserialized.effects[0].params[0].name, "radius");
        assert_eq!(deserialized.effects[0].params[0].default_value, 5.0);
        assert_eq!(deserialized.effects[0].params[0].min_value, 0.0);
        assert_eq!(deserialized.effects[0].params[0].max_value, 100.0);
    }

    #[test]
    fn plugin_effect_def_construction() {
        let effect = PluginEffectDef {
            id: "color_shift".into(),
            name: "Color Shift".into(),
            params: vec![
                PluginParamDef {
                    name: "hue".into(),
                    default_value: 0.0,
                    min_value: -180.0,
                    max_value: 180.0,
                },
                PluginParamDef {
                    name: "saturation".into(),
                    default_value: 1.0,
                    min_value: 0.0,
                    max_value: 2.0,
                },
            ],
        };

        assert_eq!(effect.id, "color_shift");
        assert_eq!(effect.name, "Color Shift");
        assert_eq!(effect.params.len(), 2);
        assert_eq!(effect.params[0].name, "hue");
        assert_eq!(effect.params[1].name, "saturation");
    }

    #[test]
    fn plugin_manifest_empty_effects() {
        let manifest = PluginManifest {
            id: "com.example.noop".into(),
            name: "No-op Plugin".into(),
            version: "0.1.0".into(),
            description: "A plugin with no effects".into(),
            effects: vec![],
        };

        let json = serde_json::to_string(&manifest).expect("serialize");
        let deserialized: PluginManifest = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.effects.is_empty());
    }
}
