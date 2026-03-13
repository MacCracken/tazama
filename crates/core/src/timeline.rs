use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::clip::{Clip, ClipId};

#[derive(Debug, Error)]
pub enum TimelineError {
    #[error("track not found: {0:?}")]
    TrackNotFound(TrackId),
    #[error("clip not found: {0:?}")]
    ClipNotFound(ClipId),
    #[error("clip overlap at frame {0}")]
    ClipOverlap(u64),
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
        }
    }

    pub fn add_clip(&mut self, clip: Clip) -> Result<(), TimelineError> {
        // TODO: overlap detection
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
}

impl Timeline {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
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
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}
