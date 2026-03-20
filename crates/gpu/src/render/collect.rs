use tazama_core::{EffectKind, ProjectSettings, Timeline, TrackKind};

pub(crate) struct ActiveClip {
    pub timeline_start: u64,
    pub source_offset: u64,
    pub opacity: f32,
    pub speed_factor: f32,
    /// Keyframe track for variable speed (speed ramping).
    pub speed_keyframe_track: Option<tazama_core::KeyframeTrack>,
    pub media_path: Option<String>,
    pub effects: Vec<tazama_core::Effect>,
}

/// Collect video clips active at the given frame, ordered bottom-to-top.
/// Respects muted, visible, and solo flags.
pub(crate) fn collect_active_clips(timeline: &Timeline, frame_index: u64) -> Vec<ActiveClip> {
    let mut clips = Vec::new();

    let any_video_solo = timeline
        .tracks
        .iter()
        .any(|t| t.solo && t.kind == TrackKind::Video);

    for track in &timeline.tracks {
        if track.muted || track.kind != TrackKind::Video || !track.visible {
            continue;
        }
        if any_video_solo && !track.solo {
            continue;
        }

        for clip in &track.clips {
            let clip_end = clip.timeline_start + clip.duration;
            if frame_index >= clip.timeline_start && frame_index < clip_end {
                let mut speed_factor = 1.0f32;
                let mut speed_keyframe_track = None;

                for e in &clip.effects {
                    if let EffectKind::Speed { factor } = &e.kind {
                        speed_factor = *factor;
                        // Check if this speed effect has keyframe tracks
                        if let Some(kt) =
                            e.keyframe_tracks.iter().find(|kt| kt.parameter == "factor")
                        {
                            speed_keyframe_track = Some(kt.clone());
                        }
                    }
                }

                clips.push(ActiveClip {
                    timeline_start: clip.timeline_start,
                    source_offset: clip.source_offset,
                    opacity: clip.opacity,
                    speed_factor,
                    speed_keyframe_track,
                    media_path: clip.media.as_ref().map(|m| m.path.clone()),
                    effects: clip.effects.clone(),
                });
            }
        }
    }

    clips
}

pub(crate) fn frame_index_to_ns(frame_index: u64, settings: &ProjectSettings) -> u64 {
    let fps = settings.frame_rate.numerator as f64 / settings.frame_rate.denominator as f64;
    (frame_index as f64 / fps * 1_000_000_000.0) as u64
}
