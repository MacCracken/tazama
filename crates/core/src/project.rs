use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::timeline::Timeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            frame_rate: FrameRate::new(30, 1),
            sample_rate: 48000,
            channels: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        assert!(denominator > 0, "frame rate denominator must be > 0");
        Self {
            numerator,
            denominator,
        }
    }

    pub fn fps(&self) -> f64 {
        if self.denominator == 0 {
            return 0.0;
        }
        self.numerator as f64 / self.denominator as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub settings: ProjectSettings,
    pub timeline: Timeline,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: impl Into<String>, settings: ProjectSettings) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            name: name.into(),
            settings,
            timeline: Timeline::new(),
            created_at: now,
            modified_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rate_fps_30() {
        let fr = FrameRate::new(30, 1);
        assert!((fr.fps() - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn frame_rate_fps_29_97() {
        let fr = FrameRate::new(30000, 1001);
        assert!((fr.fps() - 29.97002997).abs() < 0.001);
    }

    #[test]
    #[should_panic(expected = "frame rate denominator must be > 0")]
    fn frame_rate_zero_denominator_panics() {
        FrameRate::new(30, 0);
    }

    #[test]
    fn default_settings_are_1080p_30fps() {
        let s = ProjectSettings::default();
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
        assert_eq!(s.frame_rate.numerator, 30);
        assert_eq!(s.frame_rate.denominator, 1);
        assert_eq!(s.sample_rate, 48000);
        assert_eq!(s.channels, 2);
    }

    #[test]
    fn project_new_has_empty_timeline() {
        let p = Project::new("test", ProjectSettings::default());
        assert_eq!(p.name, "test");
        assert!(p.timeline.tracks.is_empty());
        assert!(p.timeline.markers.is_empty());
    }

    #[test]
    fn project_ids_are_unique() {
        let p1 = Project::new("a", ProjectSettings::default());
        let p2 = Project::new("b", ProjectSettings::default());
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn project_id_default() {
        let id1 = ProjectId::default();
        let id2 = ProjectId::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn project_new_sets_created_and_modified_at() {
        let before = Utc::now();
        let p = Project::new("timestamps", ProjectSettings::default());
        let after = Utc::now();
        assert!(p.created_at >= before && p.created_at <= after);
        assert!(p.modified_at >= before && p.modified_at <= after);
        assert_eq!(p.created_at, p.modified_at);
    }

    #[test]
    fn project_settings_default_values() {
        let s = ProjectSettings::default();
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
        assert!((s.frame_rate.fps() - 30.0).abs() < f64::EPSILON);
        assert_eq!(s.sample_rate, 48000);
        assert_eq!(s.channels, 2);
    }

    #[test]
    fn frame_rate_fps_23_976() {
        let fr = FrameRate::new(24000, 1001);
        assert!((fr.fps() - 23.976).abs() < 0.001);
    }

    #[test]
    fn frame_rate_fps_60() {
        let fr = FrameRate::new(60, 1);
        assert!((fr.fps() - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn project_serde_round_trip() {
        let p = Project::new("serde test", ProjectSettings::default());
        let json = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, p.id);
        assert_eq!(back.name, "serde test");
    }
}
