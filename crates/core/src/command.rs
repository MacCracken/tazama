use serde::{Deserialize, Serialize};

use crate::clip::{Clip, ClipId};
use crate::effect::Effect;
use crate::keyframe::KeyframeTrack;
use crate::marker::Marker;
use crate::multicam::MultiCamGroup;
use crate::timeline::{Timeline, TimelineError, TrackId, TrackKind};

/// Each variant stores enough state for both `apply()` and `undo()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditCommand {
    AddClip {
        track_id: TrackId,
        clip: Clip,
    },
    RemoveClip {
        track_id: TrackId,
        clip: Clip,
    },
    MoveClip {
        track_id: TrackId,
        clip_id: ClipId,
        old_start: u64,
        new_start: u64,
    },
    TrimClip {
        track_id: TrackId,
        clip_id: ClipId,
        old_offset: u64,
        old_duration: u64,
        new_offset: u64,
        new_duration: u64,
    },
    SplitClip {
        track_id: TrackId,
        original_id: ClipId,
        new_clip_id: ClipId,
        original_duration: u64,
        split_frame: u64,
    },
    AddTrack {
        track_id: TrackId,
        name: String,
        kind: TrackKind,
    },
    RemoveTrack {
        track_id: TrackId,
        index: usize,
        name: String,
        kind: TrackKind,
        clips: Vec<Clip>,
    },
    ApplyEffect {
        track_id: TrackId,
        clip_id: ClipId,
        effect: Effect,
    },
    RemoveEffect {
        track_id: TrackId,
        clip_id: ClipId,
        effect: Effect,
    },
    AddMarker {
        marker: Marker,
    },
    RemoveMarker {
        marker: Marker,
    },
    SetTrackVolume {
        track_id: TrackId,
        old_volume: f32,
        new_volume: f32,
    },
    SetTrackPan {
        track_id: TrackId,
        old_pan: f32,
        new_pan: f32,
    },
    SetKeyframes {
        track_id: TrackId,
        clip_id: ClipId,
        effect_index: usize,
        old_tracks: Vec<KeyframeTrack>,
        new_tracks: Vec<KeyframeTrack>,
    },
    CreateMultiCamGroup {
        group: MultiCamGroup,
    },
    SwitchAngle {
        track_id: TrackId,
        /// The clip that was on the output track before the switch (for undo).
        old_clip: Option<Box<Clip>>,
        /// The new clip placed on the output track.
        new_clip: Box<Clip>,
    },
}

impl EditCommand {
    pub fn apply(&self, timeline: &mut Timeline) -> Result<(), TimelineError> {
        match self {
            EditCommand::AddClip { track_id, clip } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.add_clip(clip.clone())
            }
            EditCommand::RemoveClip { track_id, clip } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.remove_clip(clip.id)?;
                Ok(())
            }
            EditCommand::MoveClip {
                track_id,
                clip_id,
                new_start,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.move_clip(*clip_id, *new_start)
            }
            EditCommand::TrimClip {
                track_id,
                clip_id,
                new_offset,
                new_duration,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.trim_clip(*clip_id, *new_offset, *new_duration)
            }
            EditCommand::SplitClip {
                track_id,
                original_id,
                split_frame,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.split_clip(*original_id, *split_frame)?;
                Ok(())
            }
            EditCommand::AddTrack { name, kind, .. } => {
                let mut track = crate::timeline::Track::new(name.clone(), *kind);
                // Use the stored track_id so undo can find it
                track.id = match self {
                    EditCommand::AddTrack { track_id, .. } => *track_id,
                    _ => unreachable!(),
                };
                timeline.add_track(track);
                Ok(())
            }
            EditCommand::RemoveTrack { track_id, .. } => {
                timeline.remove_track(*track_id)?;
                Ok(())
            }
            EditCommand::ApplyEffect {
                track_id,
                clip_id,
                effect,
            } => {
                let (_, clip) = timeline
                    .find_clip_mut(*clip_id)
                    .ok_or(TimelineError::ClipNotFound(*clip_id))?;
                let _ = track_id; // verified via find_clip_mut
                clip.effects.push(effect.clone());
                Ok(())
            }
            EditCommand::RemoveEffect {
                clip_id, effect, ..
            } => {
                let (_, clip) = timeline
                    .find_clip_mut(*clip_id)
                    .ok_or(TimelineError::ClipNotFound(*clip_id))?;
                clip.effects.retain(|e| e.id != effect.id);
                Ok(())
            }
            EditCommand::AddMarker { marker } => {
                timeline.add_marker(marker.clone());
                Ok(())
            }
            EditCommand::RemoveMarker { marker } => {
                timeline.remove_marker(marker.id);
                Ok(())
            }
            EditCommand::SetTrackVolume {
                track_id,
                new_volume,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.volume = *new_volume;
                Ok(())
            }
            EditCommand::SetTrackPan {
                track_id, new_pan, ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.pan = *new_pan;
                Ok(())
            }
            EditCommand::SetKeyframes {
                clip_id,
                effect_index,
                new_tracks,
                ..
            } => {
                let (_, clip) = timeline
                    .find_clip_mut(*clip_id)
                    .ok_or(TimelineError::ClipNotFound(*clip_id))?;
                if let Some(effect) = clip.effects.get_mut(*effect_index) {
                    effect.keyframe_tracks = new_tracks.clone();
                }
                Ok(())
            }
            EditCommand::CreateMultiCamGroup { group } => {
                timeline.multicam_groups.push(group.clone());
                Ok(())
            }
            EditCommand::SwitchAngle {
                track_id,
                new_clip,
                old_clip,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                // Remove old clip if present
                if let Some(old) = old_clip {
                    track.clips.retain(|c| c.id != old.id);
                }
                track.add_clip(*new_clip.clone())
            }
        }
    }

    pub fn undo(&self, timeline: &mut Timeline) -> Result<(), TimelineError> {
        match self {
            EditCommand::AddClip { track_id, clip } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.remove_clip(clip.id)?;
                Ok(())
            }
            EditCommand::RemoveClip { track_id, clip } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.add_clip(clip.clone())
            }
            EditCommand::MoveClip {
                track_id,
                clip_id,
                old_start,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.move_clip(*clip_id, *old_start)
            }
            EditCommand::TrimClip {
                track_id,
                clip_id,
                old_offset,
                old_duration,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.trim_clip(*clip_id, *old_offset, *old_duration)
            }
            EditCommand::SplitClip {
                track_id,
                original_id,
                new_clip_id,
                original_duration,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                // Remove the new clip created by the split
                track.remove_clip(*new_clip_id)?;
                // Restore original clip's duration
                let clip = track
                    .clips
                    .iter_mut()
                    .find(|c| c.id == *original_id)
                    .ok_or(TimelineError::ClipNotFound(*original_id))?;
                clip.duration = *original_duration;
                Ok(())
            }
            EditCommand::AddTrack { track_id, .. } => {
                timeline.remove_track(*track_id)?;
                Ok(())
            }
            EditCommand::RemoveTrack {
                track_id,
                index,
                name,
                kind,
                clips,
            } => {
                let mut track = crate::timeline::Track::new(name.clone(), *kind);
                track.id = *track_id;
                track.clips = clips.clone();
                // Re-insert at original index
                let idx = (*index).min(timeline.tracks.len());
                timeline.tracks.insert(idx, track);
                Ok(())
            }
            EditCommand::ApplyEffect {
                clip_id, effect, ..
            } => {
                let (_, clip) = timeline
                    .find_clip_mut(*clip_id)
                    .ok_or(TimelineError::ClipNotFound(*clip_id))?;
                clip.effects.retain(|e| e.id != effect.id);
                Ok(())
            }
            EditCommand::RemoveEffect {
                clip_id, effect, ..
            } => {
                let (_, clip) = timeline
                    .find_clip_mut(*clip_id)
                    .ok_or(TimelineError::ClipNotFound(*clip_id))?;
                clip.effects.push(effect.clone());
                Ok(())
            }
            EditCommand::AddMarker { marker } => {
                timeline.remove_marker(marker.id);
                Ok(())
            }
            EditCommand::RemoveMarker { marker } => {
                timeline.add_marker(marker.clone());
                Ok(())
            }
            EditCommand::SetTrackVolume {
                track_id,
                old_volume,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.volume = *old_volume;
                Ok(())
            }
            EditCommand::SetTrackPan {
                track_id, old_pan, ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.pan = *old_pan;
                Ok(())
            }
            EditCommand::SetKeyframes {
                clip_id,
                effect_index,
                old_tracks,
                ..
            } => {
                let (_, clip) = timeline
                    .find_clip_mut(*clip_id)
                    .ok_or(TimelineError::ClipNotFound(*clip_id))?;
                if let Some(effect) = clip.effects.get_mut(*effect_index) {
                    effect.keyframe_tracks = old_tracks.clone();
                }
                Ok(())
            }
            EditCommand::CreateMultiCamGroup { group } => {
                timeline.multicam_groups.retain(|g| g.id != group.id);
                Ok(())
            }
            EditCommand::SwitchAngle {
                track_id,
                old_clip,
                new_clip,
                ..
            } => {
                let track = timeline
                    .track_mut(*track_id)
                    .ok_or(TimelineError::TrackNotFound(*track_id))?;
                track.clips.retain(|c| c.id != new_clip.id);
                if let Some(old) = old_clip {
                    let _ = track.add_clip(*old.clone());
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct EditHistory {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
}

impl EditHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute a command, pushing it onto the undo stack and clearing redo.
    pub fn execute(
        &mut self,
        cmd: EditCommand,
        timeline: &mut Timeline,
    ) -> Result<(), TimelineError> {
        cmd.apply(timeline)?;
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        Ok(())
    }

    pub fn undo(&mut self, timeline: &mut Timeline) -> Result<bool, TimelineError> {
        if let Some(cmd) = self.undo_stack.pop() {
            cmd.undo(timeline)?;
            self.redo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn redo(&mut self, timeline: &mut Timeline) -> Result<bool, TimelineError> {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.apply(timeline)?;
            self.undo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{Clip, ClipKind};

    fn setup() -> (Timeline, EditHistory) {
        let mut timeline = Timeline::new();
        let track = crate::timeline::Track::new("V1", TrackKind::Video);
        timeline.add_track(track);
        (timeline, EditHistory::new())
    }

    #[test]
    fn add_undo_redo_cycle() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;

        let cmd = EditCommand::AddClip {
            track_id,
            clip: clip.clone(),
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 0);

        history.redo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);
        assert_eq!(timeline.tracks[0].clips[0].id, clip_id);
    }

    #[test]
    fn split_undo_restores_original() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 60);
        let clip_id = clip.id;

        // Add clip first
        let add_cmd = EditCommand::AddClip {
            track_id,
            clip: clip.clone(),
        };
        history.execute(add_cmd, &mut timeline).unwrap();

        // Now split it — we need to record the new_clip_id after split
        // For the command pattern, we need to know the new_clip_id ahead of time
        // So we do the split via the track and wrap it in a command
        let _new_clip_id = {
            let track = timeline.track_mut(track_id).unwrap();
            // Find the new clip id after split
            let right = track
                .clips
                .iter()
                .find(|c| c.id == clip_id)
                .unwrap()
                .clone();
            // Undo the direct mutation — we'll use the command system
            drop(right);
            // Actually, let's use the command directly
            ClipId::new() // placeholder — the split_clip on track generates its own
        };

        // Reset — re-do properly: we need to use the track-level split which generates the id
        // Let's just test via direct EditCommand
        // First undo the add so we can redo cleanly
        history.undo(&mut timeline).unwrap();
        history.redo(&mut timeline).unwrap();

        // Split using track method directly, then record command
        let track = timeline.track_mut(track_id).unwrap();
        let original_duration = track
            .clips
            .iter()
            .find(|c| c.id == clip_id)
            .unwrap()
            .duration;
        let new_id = track.split_clip(clip_id, 30).unwrap();

        // Now manually push the command to undo stack
        let split_cmd = EditCommand::SplitClip {
            track_id,
            original_id: clip_id,
            new_clip_id: new_id,
            original_duration,
            split_frame: 30,
        };

        // Verify split happened
        assert_eq!(timeline.tracks[0].clips.len(), 2);
        let left = timeline.tracks[0]
            .clips
            .iter()
            .find(|c| c.id == clip_id)
            .unwrap();
        assert_eq!(left.duration, 30);
        let right = timeline.tracks[0]
            .clips
            .iter()
            .find(|c| c.id == new_id)
            .unwrap();
        assert_eq!(right.duration, 30);

        // Undo the split
        split_cmd.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);
        assert_eq!(timeline.tracks[0].clips[0].duration, 60);
    }

    #[test]
    fn move_clip_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;
        history
            .execute(EditCommand::AddClip { track_id, clip }, &mut timeline)
            .unwrap();

        let cmd = EditCommand::MoveClip {
            track_id,
            clip_id,
            old_start: 0,
            new_start: 50,
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].timeline_start, 50);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].timeline_start, 0);
    }

    #[test]
    fn add_and_remove_track_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = crate::timeline::TrackId::new();
        let cmd = EditCommand::AddTrack {
            track_id,
            name: "A1".to_string(),
            kind: TrackKind::Audio,
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks.len(), 2);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks.len(), 1);
        assert!(timeline.track(track_id).is_none());
    }

    #[test]
    fn add_marker_undo() {
        let (mut timeline, mut history) = setup();
        let marker = crate::marker::Marker::new("m1", 10, crate::marker::MarkerColor::Blue);
        let marker_id = marker.id;
        let cmd = EditCommand::AddMarker { marker };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.markers.len(), 1);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.markers.len(), 0);

        history.redo(&mut timeline).unwrap();
        assert_eq!(timeline.markers.len(), 1);
        assert_eq!(timeline.markers[0].id, marker_id);
    }

    #[test]
    fn undo_empty_history_returns_false() {
        let (mut timeline, mut history) = setup();
        assert!(!history.can_undo());
        let result = history.undo(&mut timeline).unwrap();
        assert!(!result);
    }

    #[test]
    fn redo_empty_history_returns_false() {
        let (mut timeline, mut history) = setup();
        assert!(!history.can_redo());
        let result = history.redo(&mut timeline).unwrap();
        assert!(!result);
    }

    #[test]
    fn remove_clip_undo_re_adds() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;
        history
            .execute(
                EditCommand::AddClip {
                    track_id,
                    clip: clip.clone(),
                },
                &mut timeline,
            )
            .unwrap();

        let cmd = EditCommand::RemoveClip {
            track_id,
            clip: clip.clone(),
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 0);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);
        assert_eq!(timeline.tracks[0].clips[0].id, clip_id);
    }

    #[test]
    fn trim_clip_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 60);
        let clip_id = clip.id;
        history
            .execute(EditCommand::AddClip { track_id, clip }, &mut timeline)
            .unwrap();

        let cmd = EditCommand::TrimClip {
            track_id,
            clip_id,
            old_offset: 0,
            old_duration: 60,
            new_offset: 10,
            new_duration: 40,
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].source_offset, 10);
        assert_eq!(timeline.tracks[0].clips[0].duration, 40);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].source_offset, 0);
        assert_eq!(timeline.tracks[0].clips[0].duration, 60);
    }

    #[test]
    fn apply_effect_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;
        history
            .execute(EditCommand::AddClip { track_id, clip }, &mut timeline)
            .unwrap();

        let effect = crate::effect::Effect::new(crate::effect::EffectKind::Speed { factor: 2.0 });
        let effect_id = effect.id;
        let cmd = EditCommand::ApplyEffect {
            track_id,
            clip_id,
            effect,
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].effects.len(), 1);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].effects.len(), 0);

        history.redo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].effects.len(), 1);
        assert_eq!(timeline.tracks[0].clips[0].effects[0].id, effect_id);
    }

    #[test]
    fn remove_effect_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;
        history
            .execute(
                EditCommand::AddClip {
                    track_id,
                    clip: clip.clone(),
                },
                &mut timeline,
            )
            .unwrap();

        let effect =
            crate::effect::Effect::new(crate::effect::EffectKind::Volume { gain_db: -6.0 });
        // Apply effect first
        history
            .execute(
                EditCommand::ApplyEffect {
                    track_id,
                    clip_id,
                    effect: effect.clone(),
                },
                &mut timeline,
            )
            .unwrap();

        // Remove effect
        history
            .execute(
                EditCommand::RemoveEffect {
                    track_id,
                    clip_id,
                    effect: effect.clone(),
                },
                &mut timeline,
            )
            .unwrap();
        assert_eq!(timeline.tracks[0].clips[0].effects.len(), 0);

        // Undo remove — effect is back
        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips[0].effects.len(), 1);
    }

    #[test]
    fn remove_track_undo_restores_at_index() {
        let (mut timeline, mut history) = setup();
        // Add a second track
        let track_id2 = crate::timeline::TrackId::new();
        history
            .execute(
                EditCommand::AddTrack {
                    track_id: track_id2,
                    name: "A1".to_string(),
                    kind: TrackKind::Audio,
                },
                &mut timeline,
            )
            .unwrap();
        assert_eq!(timeline.tracks.len(), 2);

        // Remove it
        let cmd = EditCommand::RemoveTrack {
            track_id: track_id2,
            index: 1,
            name: "A1".to_string(),
            kind: TrackKind::Audio,
            clips: vec![],
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.tracks.len(), 1);

        // Undo — track is back at index 1
        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks.len(), 2);
        assert_eq!(timeline.tracks[1].name, "A1");
    }

    #[test]
    fn remove_marker_undo() {
        let (mut timeline, mut history) = setup();
        let marker = crate::marker::Marker::new("m1", 10, crate::marker::MarkerColor::Yellow);
        let marker_id = marker.id;
        history
            .execute(
                EditCommand::AddMarker {
                    marker: marker.clone(),
                },
                &mut timeline,
            )
            .unwrap();

        let cmd = EditCommand::RemoveMarker { marker };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.markers.len(), 0);

        history.undo(&mut timeline).unwrap();
        assert_eq!(timeline.markers.len(), 1);
        assert_eq!(timeline.markers[0].id, marker_id);
    }

    #[test]
    fn set_track_volume_nonexistent_track_returns_error() {
        let (mut timeline, _) = setup();
        let fake_id = crate::timeline::TrackId::new();
        let cmd = EditCommand::SetTrackVolume {
            track_id: fake_id,
            old_volume: 1.0,
            new_volume: 0.5,
        };
        let result = cmd.apply(&mut timeline);
        assert!(result.is_err());
    }

    #[test]
    fn set_track_pan_nonexistent_track_returns_error() {
        let (mut timeline, _) = setup();
        let fake_id = crate::timeline::TrackId::new();
        let cmd = EditCommand::SetTrackPan {
            track_id: fake_id,
            old_pan: 0.0,
            new_pan: -1.0,
        };
        let result = cmd.apply(&mut timeline);
        assert!(result.is_err());
    }

    #[test]
    fn set_keyframes_nonexistent_clip_returns_error() {
        let (mut timeline, _) = setup();
        let track_id = timeline.tracks[0].id;
        let fake_clip_id = crate::clip::ClipId::new();
        let cmd = EditCommand::SetKeyframes {
            track_id,
            clip_id: fake_clip_id,
            effect_index: 0,
            old_tracks: vec![],
            new_tracks: vec![],
        };
        let result = cmd.apply(&mut timeline);
        assert!(result.is_err());
    }

    #[test]
    fn set_keyframes_out_of_bounds_effect_index_succeeds_silently() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;
        history
            .execute(EditCommand::AddClip { track_id, clip }, &mut timeline)
            .unwrap();
        // No effects on clip, but effect_index=99 — should succeed silently
        let cmd = EditCommand::SetKeyframes {
            track_id,
            clip_id,
            effect_index: 99,
            old_tracks: vec![],
            new_tracks: vec![crate::keyframe::KeyframeTrack::new("test")],
        };
        let result = cmd.apply(&mut timeline);
        assert!(result.is_ok());
        // Clip effects unchanged (still empty)
        assert!(timeline.tracks[0].clips[0].effects.is_empty());
    }

    #[test]
    fn switch_angle_apply_and_undo() {
        let (mut timeline, _) = setup();
        let track_id = timeline.tracks[0].id;
        let old_clip = Clip::new("angle1", ClipKind::Video, 0, 30);
        let old_clip_id = old_clip.id;
        // Add old clip directly
        timeline
            .track_mut(track_id)
            .unwrap()
            .add_clip(old_clip.clone())
            .unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);

        let new_clip = Clip::new("angle2", ClipKind::Video, 0, 30);
        let new_clip_id = new_clip.id;
        let cmd = EditCommand::SwitchAngle {
            track_id,
            old_clip: Some(Box::new(old_clip)),
            new_clip: Box::new(new_clip),
        };
        cmd.apply(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);
        assert_eq!(timeline.tracks[0].clips[0].id, new_clip_id);

        // Undo
        cmd.undo(&mut timeline).unwrap();
        assert_eq!(timeline.tracks[0].clips.len(), 1);
        assert_eq!(timeline.tracks[0].clips[0].id, old_clip_id);
    }

    #[test]
    fn redo_stack_cleared_on_new_action() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;

        let clip1 = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip2 = Clip::new("clip2", ClipKind::Video, 30, 30);

        history
            .execute(
                EditCommand::AddClip {
                    track_id,
                    clip: clip1,
                },
                &mut timeline,
            )
            .unwrap();

        history.undo(&mut timeline).unwrap();
        assert!(history.can_redo());

        // New action should clear redo stack
        history
            .execute(
                EditCommand::AddClip {
                    track_id,
                    clip: clip2,
                },
                &mut timeline,
            )
            .unwrap();
        assert!(!history.can_redo());
    }

    #[test]
    fn set_track_volume_apply_and_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        assert_eq!(timeline.tracks[0].volume, 1.0);

        let cmd = EditCommand::SetTrackVolume {
            track_id,
            old_volume: 1.0,
            new_volume: 0.5,
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert!((timeline.tracks[0].volume - 0.5).abs() < 1e-6);

        history.undo(&mut timeline).unwrap();
        assert!((timeline.tracks[0].volume - 1.0).abs() < 1e-6);

        history.redo(&mut timeline).unwrap();
        assert!((timeline.tracks[0].volume - 0.5).abs() < 1e-6);
    }

    #[test]
    fn set_track_pan_apply_and_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        assert_eq!(timeline.tracks[0].pan, 0.0);

        let cmd = EditCommand::SetTrackPan {
            track_id,
            old_pan: 0.0,
            new_pan: -0.75,
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert!((timeline.tracks[0].pan - (-0.75)).abs() < 1e-6);

        history.undo(&mut timeline).unwrap();
        assert!((timeline.tracks[0].pan - 0.0).abs() < 1e-6);
    }

    #[test]
    fn set_keyframes_apply_and_undo() {
        let (mut timeline, mut history) = setup();
        let track_id = timeline.tracks[0].id;
        let clip = Clip::new("clip1", ClipKind::Video, 0, 30);
        let clip_id = clip.id;
        history
            .execute(EditCommand::AddClip { track_id, clip }, &mut timeline)
            .unwrap();

        // Apply an effect first so we can set keyframes on it
        let effect = crate::effect::Effect::new(crate::effect::EffectKind::Speed { factor: 1.0 });
        history
            .execute(
                EditCommand::ApplyEffect {
                    track_id,
                    clip_id,
                    effect,
                },
                &mut timeline,
            )
            .unwrap();

        let old_tracks = vec![];
        let mut new_track = crate::keyframe::KeyframeTrack::new("speed");
        new_track.add_keyframe(crate::keyframe::Keyframe::new(
            0,
            1.0,
            crate::keyframe::Interpolation::Linear,
        ));
        new_track.add_keyframe(crate::keyframe::Keyframe::new(
            30,
            2.0,
            crate::keyframe::Interpolation::Linear,
        ));
        let new_tracks = vec![new_track];

        let cmd = EditCommand::SetKeyframes {
            track_id,
            clip_id,
            effect_index: 0,
            old_tracks: old_tracks.clone(),
            new_tracks: new_tracks.clone(),
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(
            timeline.tracks[0].clips[0].effects[0].keyframe_tracks.len(),
            1
        );
        assert_eq!(
            timeline.tracks[0].clips[0].effects[0].keyframe_tracks[0]
                .keyframes
                .len(),
            2
        );

        history.undo(&mut timeline).unwrap();
        assert_eq!(
            timeline.tracks[0].clips[0].effects[0].keyframe_tracks.len(),
            0
        );
    }

    #[test]
    fn create_multicam_group_apply_and_undo() {
        let (mut timeline, mut history) = setup();
        assert!(timeline.multicam_groups.is_empty());

        let mut group = crate::multicam::MultiCamGroup::new("Concert");
        let track_id = timeline.tracks[0].id;
        group.add_angle(track_id, 0);
        let group_id = group.id;

        let cmd = EditCommand::CreateMultiCamGroup {
            group: group.clone(),
        };
        history.execute(cmd, &mut timeline).unwrap();
        assert_eq!(timeline.multicam_groups.len(), 1);
        assert_eq!(timeline.multicam_groups[0].name, "Concert");

        history.undo(&mut timeline).unwrap();
        assert!(timeline.multicam_groups.is_empty());

        history.redo(&mut timeline).unwrap();
        assert_eq!(timeline.multicam_groups.len(), 1);
        assert_eq!(timeline.multicam_groups[0].id, group_id);
    }
}
