use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::timeline::TrackId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MultiCamGroupId(pub Uuid);

impl MultiCamGroupId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MultiCamGroupId {
    fn default() -> Self {
        Self::new()
    }
}

/// A multi-camera group: multiple angles (tracks) synced together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCamGroup {
    pub id: MultiCamGroupId,
    pub name: String,
    /// Each angle is a (track_id, sync_offset_in_frames) pair.
    pub angles: Vec<(TrackId, i64)>,
}

impl MultiCamGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: MultiCamGroupId::new(),
            name: name.into(),
            angles: Vec::new(),
        }
    }

    pub fn add_angle(&mut self, track_id: TrackId, sync_offset_frames: i64) {
        self.angles.push((track_id, sync_offset_frames));
    }

    /// Get the active angle's track at a given frame based on the output track's clips.
    /// Returns the TrackId of the angle that should be active.
    pub fn angle_at(&self, index: usize) -> Option<(TrackId, i64)> {
        self.angles.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multicam_group_new() {
        let group = MultiCamGroup::new("Concert");
        assert_eq!(group.name, "Concert");
        assert!(group.angles.is_empty());
    }

    #[test]
    fn multicam_group_add_angles() {
        let mut group = MultiCamGroup::new("test");
        let t1 = TrackId::new();
        let t2 = TrackId::new();
        group.add_angle(t1, 0);
        group.add_angle(t2, -5);
        assert_eq!(group.angles.len(), 2);
        assert_eq!(group.angles[0], (t1, 0));
        assert_eq!(group.angles[1], (t2, -5));
    }

    #[test]
    fn multicam_group_angle_at() {
        let mut group = MultiCamGroup::new("test");
        let t1 = TrackId::new();
        group.add_angle(t1, 10);
        assert_eq!(group.angle_at(0), Some((t1, 10)));
        assert_eq!(group.angle_at(1), None);
    }

    #[test]
    fn multicam_group_serde() {
        let mut group = MultiCamGroup::new("test");
        group.add_angle(TrackId::new(), 0);
        let json = serde_json::to_string(&group).unwrap();
        let back: MultiCamGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.angles.len(), 1);
    }

    #[test]
    fn multicam_group_id_unique() {
        let g1 = MultiCamGroupId::new();
        let g2 = MultiCamGroupId::new();
        assert_ne!(g1, g2);
    }

    #[test]
    fn multicam_group_serde_multiple_angles() {
        let mut group = MultiCamGroup::new("multi-angle");
        let t1 = TrackId::new();
        let t2 = TrackId::new();
        let t3 = TrackId::new();
        group.add_angle(t1, 0);
        group.add_angle(t2, -10);
        group.add_angle(t3, 5);

        let json = serde_json::to_string(&group).unwrap();
        let back: MultiCamGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, group.id);
        assert_eq!(back.name, "multi-angle");
        assert_eq!(back.angles.len(), 3);
        assert_eq!(back.angles[0], (t1, 0));
        assert_eq!(back.angles[1], (t2, -10));
        assert_eq!(back.angles[2], (t3, 5));
    }

    #[test]
    fn multicam_group_angle_at_out_of_bounds() {
        let group = MultiCamGroup::new("empty");
        assert_eq!(group.angle_at(0), None);
        assert_eq!(group.angle_at(100), None);
        assert_eq!(group.angle_at(usize::MAX), None);
    }

    #[test]
    fn multicam_group_angle_at_various_indices() {
        let mut group = MultiCamGroup::new("test");
        let t1 = TrackId::new();
        let t2 = TrackId::new();
        let t3 = TrackId::new();
        group.add_angle(t1, 0);
        group.add_angle(t2, -5);
        group.add_angle(t3, 10);

        assert_eq!(group.angle_at(0), Some((t1, 0)));
        assert_eq!(group.angle_at(1), Some((t2, -5)));
        assert_eq!(group.angle_at(2), Some((t3, 10)));
        assert_eq!(group.angle_at(3), None);
    }

    #[test]
    fn multicam_group_id_default() {
        let id = MultiCamGroupId::default();
        // default() should produce a valid non-nil UUID
        assert_ne!(id.0, uuid::Uuid::nil());
    }
}
