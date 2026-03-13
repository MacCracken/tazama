use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effect::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipId(pub Uuid);

impl ClipId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ClipId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipKind {
    Video,
    Audio,
    Image,
    Title,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaRef {
    pub path: String,
    pub duration_frames: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub info: Option<crate::media_info::MediaInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub name: String,
    pub kind: ClipKind,
    pub media: Option<MediaRef>,
    /// Start position on the timeline in frames.
    pub timeline_start: u64,
    /// Duration on the timeline in frames.
    pub duration: u64,
    /// Offset into source media in frames (for trimming).
    pub source_offset: u64,
    pub effects: Vec<Effect>,
    pub opacity: f32,
    pub volume: f32,
}

impl Clip {
    pub fn new(
        name: impl Into<String>,
        kind: ClipKind,
        timeline_start: u64,
        duration: u64,
    ) -> Self {
        Self {
            id: ClipId::new(),
            name: name.into(),
            kind,
            media: None,
            timeline_start,
            duration,
            source_offset: 0,
            effects: Vec::new(),
            opacity: 1.0,
            volume: 1.0,
        }
    }

    pub fn with_media(mut self, media: MediaRef) -> Self {
        self.media = Some(media);
        self
    }

    pub fn timeline_end(&self) -> u64 {
        self.timeline_start + self.duration
    }
}
