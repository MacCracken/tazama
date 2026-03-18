use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackPosition {
    pub frame: u64,
    pub state: PlaybackState,
    /// Optional loop region as (start_frame, end_frame).
    pub loop_region: Option<(u64, u64)>,
}

impl PlaybackPosition {
    pub fn new() -> Self {
        Self {
            frame: 0,
            state: PlaybackState::Stopped,
            loop_region: None,
        }
    }

    pub fn seek(&mut self, frame: u64) {
        self.frame = frame;
    }

    /// Advance by one frame, wrapping at `timeline_duration` or loop region end.
    pub fn advance(&mut self, timeline_duration: u64) {
        if self.state != PlaybackState::Playing {
            return;
        }

        self.frame += 1;

        if let Some((loop_start, loop_end)) = self.loop_region {
            if self.frame >= loop_end {
                self.frame = loop_start;
            }
        } else if self.frame >= timeline_duration {
            self.frame = 0;
            self.state = PlaybackState::Stopped;
        }
    }
}

impl Default for PlaybackPosition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_sets_frame() {
        let mut pos = PlaybackPosition::new();
        pos.seek(42);
        assert_eq!(pos.frame, 42);
    }

    #[test]
    fn advance_only_when_playing() {
        let mut pos = PlaybackPosition::new();
        pos.advance(100);
        assert_eq!(pos.frame, 0);

        pos.state = PlaybackState::Playing;
        pos.advance(100);
        assert_eq!(pos.frame, 1);
    }

    #[test]
    fn advance_wraps_at_duration() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.frame = 99;
        pos.advance(100);
        assert_eq!(pos.frame, 0);
        assert_eq!(pos.state, PlaybackState::Stopped);
    }

    #[test]
    fn advance_loops_in_region() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.loop_region = Some((10, 20));
        pos.frame = 19;
        pos.advance(100);
        assert_eq!(pos.frame, 10);
        assert_eq!(pos.state, PlaybackState::Playing);
    }

    #[test]
    fn advance_does_not_stop_when_looping() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.loop_region = Some((0, 5));
        pos.frame = 4;
        pos.advance(5); // at loop end
        assert_eq!(pos.frame, 0);
        assert_eq!(pos.state, PlaybackState::Playing); // stays playing
    }

    #[test]
    fn advance_stops_at_timeline_end_without_loop() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.frame = 0;
        pos.advance(1); // duration=1, after advance frame=1 >= 1
        assert_eq!(pos.frame, 0);
        assert_eq!(pos.state, PlaybackState::Stopped);
    }

    #[test]
    fn default_is_stopped_at_zero() {
        let pos = PlaybackPosition::default();
        assert_eq!(pos.frame, 0);
        assert_eq!(pos.state, PlaybackState::Stopped);
        assert!(pos.loop_region.is_none());
    }

    #[test]
    fn advance_multiple_frames_in_loop_region() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.loop_region = Some((5, 8));
        pos.frame = 6;
        pos.advance(100); // 6 → 7
        assert_eq!(pos.frame, 7);
        pos.advance(100); // 7 → 8, wraps to 5
        assert_eq!(pos.frame, 5);
        pos.advance(100); // 5 → 6
        assert_eq!(pos.frame, 6);
    }

    #[test]
    fn seek_then_advance() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.seek(50);
        assert_eq!(pos.frame, 50);
        pos.advance(100);
        assert_eq!(pos.frame, 51);
    }

    #[test]
    fn playback_serde_round_trip() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Playing;
        pos.frame = 42;
        pos.loop_region = Some((10, 50));
        let json = serde_json::to_string(&pos).unwrap();
        let back: PlaybackPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frame, 42);
        assert_eq!(back.state, PlaybackState::Playing);
        assert_eq!(back.loop_region, Some((10, 50)));
    }

    #[test]
    fn paused_does_not_advance() {
        let mut pos = PlaybackPosition::new();
        pos.state = PlaybackState::Paused;
        pos.frame = 10;
        pos.advance(100);
        assert_eq!(pos.frame, 10);
    }
}
