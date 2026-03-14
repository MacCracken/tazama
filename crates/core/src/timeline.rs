use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::clip::{Clip, ClipId};
use crate::marker::{Marker, MarkerId};

#[derive(Debug, Error)]
pub enum TimelineError {
    #[error("track not found: {0:?}")]
    TrackNotFound(TrackId),
    #[error("clip not found: {0:?}")]
    ClipNotFound(ClipId),
    #[error("clip overlap at frame {0}")]
    ClipOverlap(u64),
    #[error("invalid split point: frame {0}")]
    InvalidSplitPoint(u64),
    #[error("invalid trim: offset={offset}, duration={duration}, max={max_duration}")]
    InvalidTrim {
        offset: u64,
        duration: u64,
        max_duration: u64,
    },
    #[error("track is locked")]
    TrackLocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub Uuid);

impl TrackId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub kind: TrackKind,
    pub clips: Vec<Clip>,
    pub muted: bool,
    pub locked: bool,
    pub solo: bool,
    pub visible: bool,
}

impl Track {
    pub fn new(name: impl Into<String>, kind: TrackKind) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind,
            clips: Vec::new(),
            muted: false,
            locked: false,
            solo: false,
            visible: true,
        }
    }

    /// Check if a clip would overlap with any existing clip on this track.
    fn check_overlap(&self, start: u64, end: u64, exclude_id: Option<ClipId>) -> Result<(), TimelineError> {
        for c in &self.clips {
            if Some(c.id) == exclude_id {
                continue;
            }
            let c_end = c.timeline_end();
            if start < c_end && end > c.timeline_start {
                return Err(TimelineError::ClipOverlap(start.max(c.timeline_start)));
            }
        }
        Ok(())
    }

    pub fn add_clip(&mut self, clip: Clip) -> Result<(), TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked);
        }
        self.check_overlap(clip.timeline_start, clip.timeline_end(), None)?;
        self.clips.push(clip);
        self.clips.sort_by_key(|c| c.timeline_start);
        Ok(())
    }

    pub fn remove_clip(&mut self, id: ClipId) -> Result<Clip, TimelineError> {
        let idx = self
            .clips
            .iter()
            .position(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;
        Ok(self.clips.remove(idx))
    }

    /// Move a clip to a new timeline start position, validating no overlaps.
    pub fn move_clip(&mut self, id: ClipId, new_start: u64) -> Result<(), TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked);
        }
        let idx = self
            .clips
            .iter()
            .position(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;

        let duration = self.clips[idx].duration;
        let new_end = new_start + duration;

        self.check_overlap(new_start, new_end, Some(id))?;

        self.clips[idx].timeline_start = new_start;
        self.clips.sort_by_key(|c| c.timeline_start);
        Ok(())
    }

    /// Split a clip at the given timeline frame. The original clip is shortened,
    /// and the new right-half clip is inserted into the track.
    pub fn split_clip(&mut self, id: ClipId, frame: u64) -> Result<ClipId, TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked);
        }
        let idx = self
            .clips
            .iter()
            .position(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;

        let right = self.clips[idx].split_at(frame)?;
        let new_id = right.id;
        self.clips.push(right);
        self.clips.sort_by_key(|c| c.timeline_start);
        Ok(new_id)
    }

    /// Trim a clip's source offset and duration.
    pub fn trim_clip(
        &mut self,
        id: ClipId,
        new_offset: u64,
        new_duration: u64,
    ) -> Result<(), TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked);
        }
        let clip = self
            .clips
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;

        let new_end = clip.timeline_start + new_duration;
        // Check overlap with the new duration (exclude self)
        let start = clip.timeline_start;
        let self_id = clip.id;
        // Release mutable borrow before calling check_overlap
        let _ = clip;
        self.check_overlap(start, new_end, Some(self_id))?;

        let clip = self
            .clips
            .iter_mut()
            .find(|c| c.id == self_id)
            .unwrap();
        clip.trim(new_offset, new_duration)
    }

    /// Duplicate a clip, placing the copy immediately after the original.
    pub fn duplicate_clip(&mut self, id: ClipId) -> Result<ClipId, TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked);
        }
        let clip = self
            .clips
            .iter()
            .find(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;

        let mut dup = clip.duplicate();
        dup.timeline_start = clip.timeline_end();
        let new_id = dup.id;

        self.check_overlap(dup.timeline_start, dup.timeline_start + dup.duration, None)?;
        self.clips.push(dup);
        self.clips.sort_by_key(|c| c.timeline_start);
        Ok(new_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub markers: Vec<Marker>,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            markers: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: Track) -> TrackId {
        let id = track.id;
        self.tracks.push(track);
        id
    }

    pub fn remove_track(&mut self, id: TrackId) -> Result<Track, TimelineError> {
        let idx = self
            .tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or(TimelineError::TrackNotFound(id))?;
        Ok(self.tracks.remove(idx))
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn track_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    pub fn duration_frames(&self) -> u64 {
        self.tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.timeline_start + c.duration)
            .max()
            .unwrap_or(0)
    }

    /// Find a clip by ID across all tracks. Returns the track ID and a reference to the clip.
    pub fn find_clip(&self, clip_id: ClipId) -> Option<(TrackId, &Clip)> {
        for track in &self.tracks {
            if let Some(clip) = track.clips.iter().find(|c| c.id == clip_id) {
                return Some((track.id, clip));
            }
        }
        None
    }

    /// Find a clip by ID across all tracks. Returns the track ID and a mutable reference.
    pub fn find_clip_mut(&mut self, clip_id: ClipId) -> Option<(TrackId, &mut Clip)> {
        for track in &mut self.tracks {
            let track_id = track.id;
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                return Some((track_id, clip));
            }
        }
        None
    }

    /// Add a marker to the timeline.
    pub fn add_marker(&mut self, marker: Marker) {
        self.markers.push(marker);
        self.markers.sort_by_key(|m| m.frame);
    }

    /// Remove a marker by ID.
    pub fn remove_marker(&mut self, id: MarkerId) -> Option<Marker> {
        let idx = self.markers.iter().position(|m| m.id == id)?;
        Some(self.markers.remove(idx))
    }

    /// Get all markers within a frame range (inclusive start, exclusive end).
    pub fn markers_in_range(&self, start: u64, end: u64) -> Vec<&Marker> {
        self.markers
            .iter()
            .filter(|m| m.frame >= start && m.frame < end)
            .collect()
    }

    /// Get tracks that should produce audio (respects muted + solo logic).
    pub fn audible_tracks(&self) -> Vec<&Track> {
        let any_solo = self.tracks.iter().any(|t| t.solo);
        self.tracks
            .iter()
            .filter(|t| {
                if t.muted {
                    return false;
                }
                if any_solo {
                    return t.solo;
                }
                true
            })
            .collect()
    }

    /// Get video tracks that should be rendered (respects visible + solo logic).
    pub fn visible_video_tracks(&self) -> Vec<&Track> {
        let any_solo = self.tracks.iter().any(|t| t.solo && t.kind == TrackKind::Video);
        self.tracks
            .iter()
            .filter(|t| {
                if t.kind != TrackKind::Video {
                    return false;
                }
                if !t.visible {
                    return false;
                }
                if any_solo {
                    return t.solo;
                }
                true
            })
            .collect()
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{Clip, ClipKind};

    fn make_clip(start: u64, duration: u64) -> Clip {
        Clip::new("test", ClipKind::Video, start, duration)
    }

    #[test]
    fn overlap_detection_rejects_overlapping_clips() {
        let mut track = Track::new("V1", TrackKind::Video);
        track.add_clip(make_clip(0, 30)).unwrap();
        let result = track.add_clip(make_clip(15, 30));
        assert!(matches!(result, Err(TimelineError::ClipOverlap(_))));
    }

    #[test]
    fn overlap_detection_allows_adjacent_clips() {
        let mut track = Track::new("V1", TrackKind::Video);
        track.add_clip(make_clip(0, 30)).unwrap();
        track.add_clip(make_clip(30, 30)).unwrap();
        assert_eq!(track.clips.len(), 2);
    }

    #[test]
    fn split_clip_offset_math() {
        let mut track = Track::new("V1", TrackKind::Video);
        let mut clip = make_clip(10, 60);
        clip.source_offset = 5;
        let clip_id = clip.id;
        track.add_clip(clip).unwrap();

        let new_id = track.split_clip(clip_id, 40).unwrap();

        let left = track.clips.iter().find(|c| c.id == clip_id).unwrap();
        assert_eq!(left.timeline_start, 10);
        assert_eq!(left.duration, 30);
        assert_eq!(left.source_offset, 5);

        let right = track.clips.iter().find(|c| c.id == new_id).unwrap();
        assert_eq!(right.timeline_start, 40);
        assert_eq!(right.duration, 30);
        assert_eq!(right.source_offset, 35); // 5 + 30
    }

    #[test]
    fn move_clip_rejects_overlap() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip_a = make_clip(0, 30);
        let clip_b = make_clip(30, 30);
        let b_id = clip_b.id;
        track.add_clip(clip_a).unwrap();
        track.add_clip(clip_b).unwrap();

        let result = track.move_clip(b_id, 15);
        assert!(matches!(result, Err(TimelineError::ClipOverlap(_))));
    }

    #[test]
    fn move_clip_succeeds_no_overlap() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 30);
        let id = clip.id;
        track.add_clip(clip).unwrap();

        track.move_clip(id, 100).unwrap();
        assert_eq!(track.clips[0].timeline_start, 100);
    }

    #[test]
    fn trim_bounds_validated() {
        use crate::clip::MediaRef;

        let mut clip = make_clip(0, 100);
        clip.media = Some(MediaRef {
            path: "test.mp4".into(),
            duration_frames: 100,
            width: None,
            height: None,
            sample_rate: None,
            channels: None,
            info: None,
        });

        // Valid trim
        clip.trim(10, 50).unwrap();
        assert_eq!(clip.source_offset, 10);
        assert_eq!(clip.duration, 50);

        // Exceeds source duration
        let result = clip.trim(50, 60);
        assert!(matches!(result, Err(TimelineError::InvalidTrim { .. })));
    }

    #[test]
    fn locked_track_rejects_mutations() {
        let mut track = Track::new("V1", TrackKind::Video);
        track.locked = true;
        let result = track.add_clip(make_clip(0, 30));
        assert!(matches!(result, Err(TimelineError::TrackLocked)));
    }

    #[test]
    fn find_clip_across_tracks() {
        let mut timeline = Timeline::new();
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 30);
        let clip_id = clip.id;
        let track_id = track.id;
        track.add_clip(clip).unwrap();
        timeline.add_track(track);

        let (found_track, found_clip) = timeline.find_clip(clip_id).unwrap();
        assert_eq!(found_track, track_id);
        assert_eq!(found_clip.id, clip_id);
    }
}
