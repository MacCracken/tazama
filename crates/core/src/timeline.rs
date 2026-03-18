use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::clip::{Clip, ClipId, ClipKind};
use crate::marker::{Marker, MarkerId};
use crate::multicam::MultiCamGroup;

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
    /// Track-level volume (0.0 to 1.0+, default 1.0).
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Track-level stereo pan (-1.0 = full left, 0.0 = center, 1.0 = full right).
    #[serde(default)]
    pub pan: f32,
}

fn default_volume() -> f32 {
    1.0
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
            volume: 1.0,
            pan: 0.0,
        }
    }

    /// Check if a clip would overlap with any existing clip on this track.
    fn check_overlap(
        &self,
        start: u64,
        end: u64,
        exclude_id: Option<ClipId>,
    ) -> Result<(), TimelineError> {
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

        let clip = self.clips.iter_mut().find(|c| c.id == self_id).unwrap();
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
    #[serde(default)]
    pub multicam_groups: Vec<MultiCamGroup>,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            markers: Vec::new(),
            multicam_groups: Vec::new(),
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

    /// Find the topmost visible video clip at a given frame position.
    ///
    /// Iterates tracks in reverse (higher tracks take priority) and returns
    /// the first active video/image clip. Respects mute, visible, and solo flags.
    pub fn topmost_video_clip_at(&self, frame: u64) -> Option<&Clip> {
        let any_video_solo = self
            .tracks
            .iter()
            .any(|t| t.solo && t.kind == TrackKind::Video);

        for track in self.tracks.iter().rev() {
            if track.muted || track.kind != TrackKind::Video || !track.visible {
                continue;
            }
            if any_video_solo && !track.solo {
                continue;
            }
            for clip in &track.clips {
                let clip_end = clip.timeline_start + clip.duration;
                if frame >= clip.timeline_start
                    && frame < clip_end
                    && matches!(clip.kind, ClipKind::Video | ClipKind::Image)
                {
                    return Some(clip);
                }
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
        let any_solo = self
            .tracks
            .iter()
            .any(|t| t.solo && t.kind == TrackKind::Video);
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
            proxy_path: None,
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

    #[test]
    fn topmost_video_clip_at_returns_clip() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        let clip = make_clip(10, 50);
        let clip_id = clip.id;
        timeline.tracks[0].add_clip(clip).unwrap();

        assert_eq!(timeline.topmost_video_clip_at(10).unwrap().id, clip_id);
        assert_eq!(timeline.topmost_video_clip_at(59).unwrap().id, clip_id);
        assert!(timeline.topmost_video_clip_at(9).is_none());
        assert!(timeline.topmost_video_clip_at(60).is_none());
    }

    #[test]
    fn topmost_video_clip_at_prefers_higher_track() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("V2", TrackKind::Video));
        let clip1 = make_clip(0, 30);
        let clip2 = make_clip(0, 30);
        let clip2_id = clip2.id;
        timeline.tracks[0].add_clip(clip1).unwrap();
        timeline.tracks[1].add_clip(clip2).unwrap();

        // Higher track (V2, index 1) takes priority
        let found = timeline.topmost_video_clip_at(0).unwrap();
        assert_eq!(found.id, clip2_id);
    }

    #[test]
    fn topmost_video_clip_at_skips_muted() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("V2", TrackKind::Video));
        let clip1 = make_clip(0, 30);
        let clip1_id = clip1.id;
        let clip2 = make_clip(0, 30);
        timeline.tracks[0].add_clip(clip1).unwrap();
        timeline.tracks[1].add_clip(clip2).unwrap();
        timeline.tracks[1].muted = true;

        // V2 is muted, so V1 is returned
        let found = timeline.topmost_video_clip_at(0).unwrap();
        assert_eq!(found.id, clip1_id);
    }

    #[test]
    fn topmost_video_clip_at_respects_solo() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("V2", TrackKind::Video));
        let clip1 = make_clip(0, 30);
        let clip1_id = clip1.id;
        let clip2 = make_clip(0, 30);
        timeline.tracks[0].add_clip(clip1).unwrap();
        timeline.tracks[1].add_clip(clip2).unwrap();
        timeline.tracks[0].solo = true;

        // V1 is solo'd, so V2 is excluded
        let found = timeline.topmost_video_clip_at(0).unwrap();
        assert_eq!(found.id, clip1_id);
    }

    #[test]
    fn topmost_video_clip_at_ignores_audio_tracks() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("A1", TrackKind::Audio));
        let clip = Clip::new("audio", ClipKind::Audio, 0, 30);
        timeline.tracks[0].add_clip(clip).unwrap();

        assert!(timeline.topmost_video_clip_at(0).is_none());
    }

    #[test]
    fn topmost_video_clip_at_skips_invisible() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        let clip = make_clip(0, 30);
        timeline.tracks[0].add_clip(clip).unwrap();
        timeline.tracks[0].visible = false;

        assert!(timeline.topmost_video_clip_at(0).is_none());
    }

    // --- Marker tests ---

    #[test]
    fn add_marker_sorted_by_frame() {
        let mut timeline = Timeline::new();
        let m1 = crate::marker::Marker::new("end", 100, crate::marker::MarkerColor::Red);
        let m2 = crate::marker::Marker::new("start", 10, crate::marker::MarkerColor::Blue);
        timeline.add_marker(m1);
        timeline.add_marker(m2);
        assert_eq!(timeline.markers.len(), 2);
        assert_eq!(timeline.markers[0].frame, 10);
        assert_eq!(timeline.markers[1].frame, 100);
    }

    #[test]
    fn remove_marker_by_id() {
        let mut timeline = Timeline::new();
        let m = crate::marker::Marker::new("test", 50, crate::marker::MarkerColor::Green);
        let mid = m.id;
        timeline.add_marker(m);
        assert_eq!(timeline.markers.len(), 1);
        let removed = timeline.remove_marker(mid).unwrap();
        assert_eq!(removed.name, "test");
        assert_eq!(timeline.markers.len(), 0);
    }

    #[test]
    fn remove_nonexistent_marker_returns_none() {
        let mut timeline = Timeline::new();
        assert!(
            timeline
                .remove_marker(crate::marker::MarkerId::new())
                .is_none()
        );
    }

    #[test]
    fn markers_in_range_filters_correctly() {
        let mut timeline = Timeline::new();
        timeline.add_marker(crate::marker::Marker::new(
            "a",
            5,
            crate::marker::MarkerColor::Red,
        ));
        timeline.add_marker(crate::marker::Marker::new(
            "b",
            10,
            crate::marker::MarkerColor::Red,
        ));
        timeline.add_marker(crate::marker::Marker::new(
            "c",
            15,
            crate::marker::MarkerColor::Red,
        ));
        timeline.add_marker(crate::marker::Marker::new(
            "d",
            20,
            crate::marker::MarkerColor::Red,
        ));

        let range = timeline.markers_in_range(10, 20);
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].name, "b");
        assert_eq!(range[1].name, "c");
    }

    // --- Duration tests ---

    #[test]
    fn duration_frames_empty_timeline() {
        let timeline = Timeline::new();
        assert_eq!(timeline.duration_frames(), 0);
    }

    #[test]
    fn duration_frames_multiple_tracks() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("A1", TrackKind::Audio));
        timeline.tracks[0].add_clip(make_clip(0, 100)).unwrap();
        timeline.tracks[1]
            .add_clip(Clip::new("audio", ClipKind::Audio, 50, 200))
            .unwrap();
        // Max end is 50 + 200 = 250
        assert_eq!(timeline.duration_frames(), 250);
    }

    // --- Clip duplicate tests ---

    #[test]
    fn duplicate_clip_gets_new_id() {
        let clip = make_clip(10, 50);
        let dup = clip.duplicate();
        assert_ne!(clip.id, dup.id);
        assert_eq!(clip.timeline_start, dup.timeline_start);
        assert_eq!(clip.duration, dup.duration);
        assert_eq!(clip.name, dup.name);
    }

    // --- Audible tracks tests ---

    #[test]
    fn audible_tracks_excludes_muted() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("A1", TrackKind::Audio));
        timeline.add_track(Track::new("A2", TrackKind::Audio));
        timeline.tracks[0].muted = true;
        let audible = timeline.audible_tracks();
        assert_eq!(audible.len(), 1);
        assert_eq!(audible[0].name, "A2");
    }

    #[test]
    fn audible_tracks_solo_mode() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("A1", TrackKind::Audio));
        timeline.add_track(Track::new("A2", TrackKind::Audio));
        timeline.tracks[1].solo = true;
        let audible = timeline.audible_tracks();
        assert_eq!(audible.len(), 1);
        assert_eq!(audible[0].name, "A2");
    }

    // --- visible_video_tracks tests ---

    #[test]
    fn visible_video_tracks_excludes_audio() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("A1", TrackKind::Audio));
        let visible = timeline.visible_video_tracks();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "V1");
    }

    #[test]
    fn visible_video_tracks_excludes_invisible() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("V2", TrackKind::Video));
        timeline.tracks[0].visible = false;
        let visible = timeline.visible_video_tracks();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "V2");
    }

    #[test]
    fn visible_video_tracks_solo() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("V2", TrackKind::Video));
        timeline.tracks[0].solo = true;
        let visible = timeline.visible_video_tracks();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "V1");
    }

    // --- Track operations ---

    #[test]
    fn remove_track_by_id() {
        let mut timeline = Timeline::new();
        let id = timeline.add_track(Track::new("V1", TrackKind::Video));
        assert_eq!(timeline.tracks.len(), 1);
        let removed = timeline.remove_track(id).unwrap();
        assert_eq!(removed.name, "V1");
        assert_eq!(timeline.tracks.len(), 0);
    }

    #[test]
    fn remove_nonexistent_track_errors() {
        let mut timeline = Timeline::new();
        let result = timeline.remove_track(TrackId::new());
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_clip_on_track() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 30);
        let clip_id = clip.id;
        track.add_clip(clip).unwrap();
        let dup_id = track.duplicate_clip(clip_id).unwrap();
        assert_eq!(track.clips.len(), 2);
        // Duplicate should start at end of original
        let dup = track.clips.iter().find(|c| c.id == dup_id).unwrap();
        assert_eq!(dup.timeline_start, 30);
        assert_eq!(dup.duration, 30);
    }

    #[test]
    fn duplicate_clip_locked_track_rejected() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 30);
        let clip_id = clip.id;
        track.add_clip(clip).unwrap();
        track.locked = true;
        let result = track.duplicate_clip(clip_id);
        assert!(matches!(result, Err(TimelineError::TrackLocked)));
    }

    #[test]
    fn trim_clip_on_track() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 60);
        let clip_id = clip.id;
        track.add_clip(clip).unwrap();
        track.trim_clip(clip_id, 10, 40).unwrap();
        assert_eq!(track.clips[0].source_offset, 10);
        assert_eq!(track.clips[0].duration, 40);
    }

    #[test]
    fn split_clip_locked_track() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 60);
        let clip_id = clip.id;
        track.add_clip(clip).unwrap();
        track.locked = true;
        let result = track.split_clip(clip_id, 30);
        assert!(matches!(result, Err(TimelineError::TrackLocked)));
    }

    #[test]
    fn trim_clip_locked_track() {
        let mut track = Track::new("V1", TrackKind::Video);
        let clip = make_clip(0, 60);
        let clip_id = clip.id;
        track.add_clip(clip).unwrap();
        track.locked = true;
        let result = track.trim_clip(clip_id, 10, 40);
        assert!(matches!(result, Err(TimelineError::TrackLocked)));
    }

    #[test]
    fn timeline_default_is_empty() {
        let t = Timeline::default();
        assert!(t.tracks.is_empty());
        assert!(t.markers.is_empty());
    }

    #[test]
    fn track_find_and_find_mut() {
        let mut timeline = Timeline::new();
        let id = timeline.add_track(Track::new("V1", TrackKind::Video));
        assert!(timeline.track(id).is_some());
        assert!(timeline.track_mut(id).is_some());
        assert!(timeline.track(TrackId::new()).is_none());
    }

    #[test]
    fn track_new_has_default_volume_and_pan() {
        let track = Track::new("V1", TrackKind::Video);
        assert!((track.volume - 1.0).abs() < 1e-6);
        assert!((track.pan - 0.0).abs() < 1e-6);
    }

    #[test]
    fn timeline_new_has_empty_multicam_groups() {
        let timeline = Timeline::new();
        assert!(timeline.multicam_groups.is_empty());
    }

    #[test]
    fn timeline_serde_backward_compat_without_volume_pan_multicam() {
        // Simulate old JSON format without volume, pan, or multicam_groups fields
        let json = r#"{
            "tracks": [{
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "V1",
                "kind": "Video",
                "clips": [],
                "muted": false,
                "locked": false,
                "solo": false,
                "visible": true
            }],
            "markers": []
        }"#;
        let timeline: Timeline = serde_json::from_str(json).unwrap();
        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(timeline.tracks[0].name, "V1");
        // volume should default to 1.0
        assert!((timeline.tracks[0].volume - 1.0).abs() < 1e-6);
        // pan should default to 0.0
        assert!((timeline.tracks[0].pan - 0.0).abs() < 1e-6);
        // multicam_groups should default to empty
        assert!(timeline.multicam_groups.is_empty());
    }

    #[test]
    fn timeline_serde_round_trip_with_multicam() {
        let mut timeline = Timeline::new();
        let track = Track::new("V1", TrackKind::Video);
        let track_id = track.id;
        timeline.add_track(track);

        let mut group = crate::multicam::MultiCamGroup::new("concert");
        group.add_angle(track_id, 0);
        timeline.multicam_groups.push(group);

        let json = serde_json::to_string(&timeline).unwrap();
        let back: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(back.multicam_groups.len(), 1);
        assert_eq!(back.multicam_groups[0].name, "concert");
    }
}
