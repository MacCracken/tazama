use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effect::Effect;
use crate::timeline::TimelineError;

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

    /// Adjust trim points. `new_source_offset` is the new offset into source media,
    /// `new_duration` is the new timeline duration.
    pub fn trim(&mut self, new_source_offset: u64, new_duration: u64) -> Result<(), TimelineError> {
        if let Some(ref media) = self.media {
            let source_end = new_source_offset + new_duration;
            if source_end > media.duration_frames {
                return Err(TimelineError::InvalidTrim {
                    offset: new_source_offset,
                    duration: new_duration,
                    max_duration: media.duration_frames,
                });
            }
        }
        if new_duration == 0 {
            return Err(TimelineError::InvalidTrim {
                offset: new_source_offset,
                duration: new_duration,
                max_duration: self
                    .media
                    .as_ref()
                    .map(|m| m.duration_frames)
                    .unwrap_or(u64::MAX),
            });
        }
        self.source_offset = new_source_offset;
        self.duration = new_duration;
        Ok(())
    }

    /// Split this clip at the given timeline frame. Returns the right half as a new clip.
    /// This clip is shortened to end at `frame`.
    pub fn split_at(&mut self, frame: u64) -> Result<Clip, TimelineError> {
        if frame <= self.timeline_start || frame >= self.timeline_end() {
            return Err(TimelineError::InvalidSplitPoint(frame));
        }

        let left_duration = frame - self.timeline_start;
        let right_duration = self.duration - left_duration;
        let right_source_offset = self.source_offset + left_duration;

        let mut right = self.clone();
        right.id = ClipId::new();
        right.timeline_start = frame;
        right.duration = right_duration;
        right.source_offset = right_source_offset;

        self.duration = left_duration;

        Ok(right)
    }

    /// Deep clone with a new ClipId.
    pub fn duplicate(&self) -> Clip {
        let mut dup = self.clone();
        dup.id = ClipId::new();
        dup
    }
}
