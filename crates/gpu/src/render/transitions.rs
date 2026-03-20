use ash::vk;
use bytemuck;
use gpu_allocator::MemoryLocation;
use tazama_core::{EffectKind, ProjectSettings, Timeline, TrackKind, TransitionKind};

use crate::buffer::GpuBuffer;
use crate::context::GpuError;
use crate::frame_source::FrameSource;
use crate::pipeline::TransitionPush;

use super::Renderer;

impl Renderer {
    pub(crate) fn apply_transitions(
        &self,
        timeline: &Timeline,
        frame_index: u64,
        accumulator: &mut GpuBuffer,
        frame_source: &dyn FrameSource,
        settings: &ProjectSettings,
    ) -> Result<(), GpuError> {
        let width = settings.width;
        let height = settings.height;
        let frame_size = GpuBuffer::frame_buffer_size(width, height);

        let any_video_solo = timeline
            .tracks
            .iter()
            .any(|t| t.solo && t.kind == TrackKind::Video);

        // Find clips that have transition effects active at this frame
        for track in &timeline.tracks {
            if track.muted || track.kind != TrackKind::Video || !track.visible {
                continue;
            }
            if any_video_solo && !track.solo {
                continue;
            }

            for (i, clip) in track.clips.iter().enumerate() {
                for effect in &clip.effects {
                    if !effect.enabled {
                        continue;
                    }
                    if let EffectKind::Transition {
                        kind: trans_kind,
                        duration_frames,
                    } = &effect.kind
                    {
                        let clip_end = clip.timeline_start + clip.duration;
                        let trans_start = clip_end.saturating_sub(*duration_frames);

                        if frame_index >= trans_start && frame_index < clip_end {
                            // Get the next clip's frame
                            let next_clip = match track.clips.get(i + 1) {
                                Some(c) => c,
                                None => continue,
                            };
                            let next_media = match &next_clip.media {
                                Some(m) => &m.path,
                                None => continue,
                            };

                            let progress =
                                (frame_index - trans_start) as f32 / *duration_frames as f32;

                            let next_source_frame = next_clip.source_offset;
                            let next_decoded =
                                frame_source.get_frame(next_media, next_source_frame)?;

                            let mut next_staging = GpuBuffer::new(
                                &self.ctx,
                                frame_size,
                                vk::BufferUsageFlags::STORAGE_BUFFER,
                                MemoryLocation::CpuToGpu,
                                "transition_next",
                            )?;
                            next_staging.write(&next_decoded.data)?;

                            let output = GpuBuffer::new(
                                &self.ctx,
                                frame_size,
                                vk::BufferUsageFlags::STORAGE_BUFFER,
                                MemoryLocation::GpuOnly,
                                "transition_out",
                            )?;

                            let push = TransitionPush {
                                width,
                                height,
                                progress,
                                _pad: 0,
                            };

                            match trans_kind {
                                TransitionKind::Dissolve => {
                                    self.dispatch_3buffer(
                                        &self.pipelines.dissolve,
                                        accumulator,
                                        &next_staging,
                                        &output,
                                        bytemuck::bytes_of(&push),
                                        width * height,
                                    )?;
                                }
                                TransitionKind::Wipe => {
                                    self.dispatch_3buffer(
                                        &self.pipelines.wipe,
                                        accumulator,
                                        &next_staging,
                                        &output,
                                        bytemuck::bytes_of(&push),
                                        width * height,
                                    )?;
                                }
                                TransitionKind::Fade => {
                                    // Fade out current frame
                                    self.dispatch_2buffer(
                                        &self.pipelines.fade,
                                        accumulator,
                                        &output,
                                        bytemuck::bytes_of(&push),
                                        width * height,
                                    )?;
                                }
                                TransitionKind::Cut => {
                                    // No transition effect for cuts
                                }
                            }

                            next_staging.destroy(&self.ctx);
                            let old = std::mem::replace(accumulator, output);
                            old.destroy(&self.ctx);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
