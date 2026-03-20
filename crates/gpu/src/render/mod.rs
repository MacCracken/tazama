mod collect;
mod dispatch;
mod effects;
mod transitions;

use std::sync::Arc;

use ash::vk;
use bytemuck;
use bytes::Bytes;
use gpu_allocator::MemoryLocation;
use tazama_core::{ProjectSettings, Timeline, keyframe};

use crate::buffer::GpuBuffer;
use crate::context::{GpuContext, GpuError};
use crate::frame_source::{FrameSource, GpuFrame};
use crate::pipeline::{CompositePush, PipelineCache};

use collect::{collect_active_clips, frame_index_to_ns};
use effects::{EffectContext, apply_effect};

/// Renders timeline frames using Vulkan compute pipelines.
pub struct Renderer {
    pub(crate) ctx: Arc<GpuContext>,
    pub(crate) pipelines: PipelineCache,
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) fence: vk::Fence,
}

impl Renderer {
    pub fn new(ctx: Arc<GpuContext>) -> Result<Self, GpuError> {
        let pipelines = PipelineCache::new(&ctx)?;

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(ctx.command_pool())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { ctx.device().allocate_command_buffers(&alloc_info)? }[0];

        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe { ctx.device().create_fence(&fence_info, None)? };

        Ok(Self {
            ctx,
            pipelines,
            command_buffer,
            fence,
        })
    }

    /// Render a single frame of the timeline at the given frame index.
    pub fn render_frame(
        &self,
        timeline: &Timeline,
        frame_index: u64,
        frame_source: &dyn FrameSource,
        settings: &ProjectSettings,
    ) -> Result<GpuFrame, GpuError> {
        let _span = tracing::debug_span!(
            "render_frame",
            frame = frame_index,
            width = settings.width,
            height = settings.height
        )
        .entered();

        let width = settings.width;
        let height = settings.height;
        let frame_size = GpuBuffer::frame_buffer_size(width, height);

        // Collect active video clips at this frame (bottom track first)
        let active_clips = collect_active_clips(timeline, frame_index);

        if active_clips.is_empty() {
            // Return transparent black frame
            return Ok(GpuFrame {
                frame_index,
                width,
                height,
                data: Bytes::from(vec![0u8; frame_size as usize]),
                timestamp_ns: frame_index_to_ns(frame_index, settings),
            });
        }

        // Accumulator buffer (starts as transparent black)
        let mut accumulator = GpuBuffer::new(
            &self.ctx,
            frame_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly,
            "accumulator",
        )?;

        // Clear accumulator to zero
        self.clear_buffer(&accumulator, frame_size)?;

        for clip_info in &active_clips {
            // Compute source frame index with speed ramping support
            let local_frame = frame_index - clip_info.timeline_start;
            let source_frame = if let Some(ref speed_track) = clip_info.speed_keyframe_track {
                // Variable speed: integrate the speed curve
                let integrated =
                    keyframe::integrated_speed(speed_track, clip_info.timeline_start, frame_index);
                clip_info.source_offset + (integrated.max(0.0) as u64)
            } else {
                // Constant speed
                let speed = clip_info.speed_factor;
                clip_info.source_offset + ((local_frame as f32 * speed).max(0.0) as u64)
            };

            // Decode source frame
            let media_path = match &clip_info.media_path {
                Some(p) => p.as_str(),
                None => continue,
            };
            let decoded = frame_source.get_frame(media_path, source_frame)?;

            // Upload decoded frame to staging buffer
            let mut staging_in = GpuBuffer::new(
                &self.ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
                "staging_in",
            )?;
            staging_in.write(&decoded.data)?;

            // Working buffer for effect chain
            let mut current_input = staging_in;
            let mut temp_buffers: Vec<GpuBuffer> = Vec::new();

            // Apply effects sequentially
            for effect in &clip_info.effects {
                if !effect.enabled {
                    continue;
                }

                let ectx = EffectContext {
                    renderer: self,
                    effect,
                    frame_index,
                    width,
                    height,
                    input_buffer: &current_input,
                    output_buffer: &current_input,
                    extra_buffer: None,
                    pixel_count: width * height,
                };
                if let Some(output) = apply_effect(&ectx)? {
                    temp_buffers.push(current_input);
                    current_input = output;
                }
            }

            // Composite onto accumulator with clip opacity
            let composite_out = GpuBuffer::new(
                &self.ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::GpuOnly,
                "composite_out",
            )?;

            let push = CompositePush {
                width,
                height,
                opacity: clip_info.opacity,
                _pad: 0,
            };

            self.dispatch_3buffer(
                &self.pipelines.composite,
                &accumulator,
                &current_input,
                &composite_out,
                bytemuck::bytes_of(&push),
                width * height,
            )?;

            // Swap accumulator
            let old_accum = std::mem::replace(&mut accumulator, composite_out);
            temp_buffers.push(old_accum);
            temp_buffers.push(current_input);

            // Clean up temp buffers
            for buf in temp_buffers {
                buf.destroy(&self.ctx);
            }
        }

        // Handle transitions between adjacent clips
        // (check for Transition effects on clips and apply dissolve/wipe/fade)
        self.apply_transitions(
            timeline,
            frame_index,
            &mut accumulator,
            frame_source,
            settings,
        )?;

        // Readback
        let readback = GpuBuffer::new(
            &self.ctx,
            frame_size,
            vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuToCpu,
            "readback",
        )?;

        self.copy_buffer(&accumulator, &readback, frame_size)?;

        let data = readback.read(frame_size as usize)?.to_vec();

        readback.destroy(&self.ctx);
        accumulator.destroy(&self.ctx);

        Ok(GpuFrame {
            frame_index,
            width,
            height,
            data: Bytes::from(data),
            timestamp_ns: frame_index_to_ns(frame_index, settings),
        })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.ctx.device().device_wait_idle();
            self.ctx.device().destroy_fence(self.fence, None);
            self.pipelines.destroy(self.ctx.device());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collect::{collect_active_clips, frame_index_to_ns};
    use tazama_core::*;

    fn test_timeline() -> Timeline {
        let mut timeline = Timeline::new();
        let track_id = timeline.add_track(Track::new("V1", TrackKind::Video));

        let clip = Clip {
            id: ClipId::new(),
            name: "test.mp4".to_string(),
            kind: ClipKind::Video,
            media: Some(MediaRef {
                path: "/tmp/test.mp4".to_string(),
                duration_frames: 300,
                width: Some(1920),
                height: Some(1080),
                sample_rate: None,
                channels: None,
                info: None,
                proxy_path: None,
            }),
            timeline_start: 0,
            duration: 150,
            source_offset: 0,
            effects: vec![],
            opacity: 1.0,
            volume: 1.0,
        };
        timeline.tracks[0].add_clip(clip).unwrap();

        let clip2 = Clip {
            id: ClipId::new(),
            name: "test2.mp4".to_string(),
            kind: ClipKind::Video,
            media: Some(MediaRef {
                path: "/tmp/test2.mp4".to_string(),
                duration_frames: 200,
                width: Some(1920),
                height: Some(1080),
                sample_rate: None,
                channels: None,
                info: None,
                proxy_path: None,
            }),
            timeline_start: 150,
            duration: 100,
            source_offset: 10,
            effects: vec![],
            opacity: 0.8,
            volume: 1.0,
        };
        timeline.tracks[0].add_clip(clip2).unwrap();

        // Add an audio track (should be ignored)
        timeline.add_track(Track::new("A1", TrackKind::Audio));

        let _ = track_id;
        timeline
    }

    #[test]
    fn test_collect_active_clips_at_frame_0() {
        let timeline = test_timeline();
        let clips = collect_active_clips(&timeline, 0);
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].media_path.as_deref(), Some("/tmp/test.mp4"));
        assert_eq!(clips[0].opacity, 1.0);
    }

    #[test]
    fn test_collect_active_clips_at_frame_150() {
        let timeline = test_timeline();
        let clips = collect_active_clips(&timeline, 150);
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].media_path.as_deref(), Some("/tmp/test2.mp4"));
        assert_eq!(clips[0].source_offset, 10);
    }

    #[test]
    fn test_collect_no_clips_past_end() {
        let timeline = test_timeline();
        let clips = collect_active_clips(&timeline, 300);
        assert_eq!(clips.len(), 0);
    }

    #[test]
    fn test_speed_factor_extraction() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        let clip = Clip {
            id: ClipId::new(),
            name: "fast.mp4".to_string(),
            kind: ClipKind::Video,
            media: Some(MediaRef {
                path: "/tmp/fast.mp4".to_string(),
                duration_frames: 100,
                width: Some(1920),
                height: Some(1080),
                sample_rate: None,
                channels: None,
                info: None,
                proxy_path: None,
            }),
            timeline_start: 0,
            duration: 50,
            source_offset: 0,
            effects: vec![Effect {
                id: EffectId::new(),
                kind: EffectKind::Speed { factor: 2.0 },
                enabled: true,
                keyframe_tracks: vec![],
            }],
            opacity: 1.0,
            volume: 1.0,
        };
        timeline.tracks[0].add_clip(clip).unwrap();

        let clips = collect_active_clips(&timeline, 10);
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].speed_factor, 2.0);
    }

    #[test]
    fn test_muted_track_ignored() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.tracks[0].muted = true;
        let clip = Clip {
            id: ClipId::new(),
            name: "muted.mp4".to_string(),
            kind: ClipKind::Video,
            media: None,
            timeline_start: 0,
            duration: 100,
            source_offset: 0,
            effects: vec![],
            opacity: 1.0,
            volume: 1.0,
        };
        timeline.tracks[0].add_clip(clip).unwrap();

        let clips = collect_active_clips(&timeline, 50);
        assert_eq!(clips.len(), 0);
    }

    #[test]
    fn test_frame_index_to_ns() {
        let settings = ProjectSettings {
            width: 1920,
            height: 1080,
            frame_rate: FrameRate {
                numerator: 30,
                denominator: 1,
            },
            sample_rate: 48000,
            channels: 2,
        };
        // Frame 30 at 30fps = 1 second = 1_000_000_000 ns
        let ns = frame_index_to_ns(30, &settings);
        assert_eq!(ns, 1_000_000_000);
    }

    #[test]
    fn test_frame_buffer_size() {
        assert_eq!(GpuBuffer::frame_buffer_size(1920, 1080), 1920 * 1080 * 4);
    }

    #[test]
    fn test_frame_buffer_size_small() {
        assert_eq!(GpuBuffer::frame_buffer_size(1, 1), 4);
        assert_eq!(GpuBuffer::frame_buffer_size(0, 0), 0);
    }

    #[test]
    fn test_frame_index_to_ns_zero() {
        let settings = ProjectSettings {
            width: 1920,
            height: 1080,
            frame_rate: FrameRate {
                numerator: 30,
                denominator: 1,
            },
            sample_rate: 48000,
            channels: 2,
        };
        assert_eq!(frame_index_to_ns(0, &settings), 0);
    }

    #[test]
    fn test_frame_index_to_ns_fractional_fps() {
        let settings = ProjectSettings {
            width: 1920,
            height: 1080,
            frame_rate: FrameRate {
                numerator: 24000,
                denominator: 1001,
            },
            sample_rate: 48000,
            channels: 2,
        };
        // 24 frames at 23.976fps ≈ 1.001 seconds
        let ns = frame_index_to_ns(24, &settings);
        // Should be approximately 1_001_000_000 ns
        assert!((ns as f64 - 1_001_000_000.0).abs() < 1_000_000.0);
    }

    #[test]
    fn test_collect_clips_with_solo_track() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        timeline.add_track(Track::new("V2", TrackKind::Video));

        let clip1 = Clip {
            id: ClipId::new(),
            name: "c1".to_string(),
            kind: ClipKind::Video,
            media: Some(MediaRef {
                path: "/tmp/a.mp4".to_string(),
                duration_frames: 100,
                width: Some(1920),
                height: Some(1080),
                sample_rate: None,
                channels: None,
                info: None,
                proxy_path: None,
            }),
            timeline_start: 0,
            duration: 100,
            source_offset: 0,
            effects: vec![],
            opacity: 1.0,
            volume: 1.0,
        };
        let clip2 = Clip {
            id: ClipId::new(),
            name: "c2".to_string(),
            kind: ClipKind::Video,
            media: Some(MediaRef {
                path: "/tmp/b.mp4".to_string(),
                duration_frames: 100,
                width: Some(1920),
                height: Some(1080),
                sample_rate: None,
                channels: None,
                info: None,
                proxy_path: None,
            }),
            timeline_start: 0,
            duration: 100,
            source_offset: 0,
            effects: vec![],
            opacity: 1.0,
            volume: 1.0,
        };

        timeline.tracks[0].add_clip(clip1).unwrap();
        timeline.tracks[1].add_clip(clip2).unwrap();

        // Without solo: both clips active
        let clips = collect_active_clips(&timeline, 50);
        assert_eq!(clips.len(), 2);

        // Solo V1: only V1 clip
        timeline.tracks[0].solo = true;
        let clips = collect_active_clips(&timeline, 50);
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].media_path.as_deref(), Some("/tmp/a.mp4"));
    }

    #[test]
    fn test_collect_clips_invisible_track_excluded() {
        let mut timeline = Timeline::new();
        timeline.add_track(Track::new("V1", TrackKind::Video));
        let clip = Clip {
            id: ClipId::new(),
            name: "c1".to_string(),
            kind: ClipKind::Video,
            media: None,
            timeline_start: 0,
            duration: 100,
            source_offset: 0,
            effects: vec![],
            opacity: 1.0,
            volume: 1.0,
        };
        timeline.tracks[0].add_clip(clip).unwrap();
        timeline.tracks[0].visible = false;

        let clips = collect_active_clips(&timeline, 50);
        assert_eq!(clips.len(), 0);
    }
}
