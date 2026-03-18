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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_new() {
        let m = Marker::new("chapter", 100, MarkerColor::Green);
        assert_eq!(m.name, "chapter");
        assert_eq!(m.frame, 100);
        assert_eq!(m.color, MarkerColor::Green);
    }

    #[test]
    fn marker_id_default() {
        let id1 = MarkerId::default();
        let id2 = MarkerId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn marker_serde_round_trip() {
        let m = Marker::new("test", 42, MarkerColor::Purple);
        let json = serde_json::to_string(&m).unwrap();
        let back: Marker = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.name, "test");
        assert_eq!(back.frame, 42);
        assert_eq!(back.color, MarkerColor::Purple);
    }

    #[test]
    fn marker_serde_all_colors_serialize_correctly() {
        let colors = [
            (MarkerColor::Red, "Red"),
            (MarkerColor::Orange, "Orange"),
            (MarkerColor::Yellow, "Yellow"),
            (MarkerColor::Green, "Green"),
            (MarkerColor::Blue, "Blue"),
            (MarkerColor::Purple, "Purple"),
            (MarkerColor::White, "White"),
        ];
        for (color, expected_str) in colors {
            let json = serde_json::to_string(&color).unwrap();
            assert!(
                json.contains(expected_str),
                "expected {expected_str} in {json}"
            );
        }
    }

    #[test]
    fn marker_serde_preserves_frame_zero() {
        let m = Marker::new("start", 0, MarkerColor::White);
        let json = serde_json::to_string(&m).unwrap();
        let back: Marker = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frame, 0);
        assert_eq!(back.name, "start");
    }

    #[test]
    fn all_marker_colors() {
        for color in [
            MarkerColor::Red,
            MarkerColor::Orange,
            MarkerColor::Yellow,
            MarkerColor::Green,
            MarkerColor::Blue,
            MarkerColor::Purple,
            MarkerColor::White,
        ] {
            let m = Marker::new("test", 0, color);
            let json = serde_json::to_string(&m).unwrap();
            let back: Marker = serde_json::from_str(&json).unwrap();
            assert_eq!(back.color, color);
        }
    }
}
