use std::sync::Arc;

use ash::vk;
use bytemuck;
use bytes::Bytes;
use gpu_allocator::MemoryLocation;
use tazama_core::{EffectKind, ProjectSettings, Timeline, TrackKind, TransitionKind};

use crate::buffer::GpuBuffer;
use crate::context::{GpuContext, GpuError};
use crate::frame_source::{FrameSource, GpuFrame};
use crate::pipeline::{ColorGradePush, CompositePush, CropPush, PipelineCache, TransitionPush};

/// Renders timeline frames using Vulkan compute pipelines.
pub struct Renderer {
    ctx: Arc<GpuContext>,
    pipelines: PipelineCache,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
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
            // Compute source frame index
            let speed = clip_info.speed_factor;
            let local_frame = frame_index - clip_info.timeline_start;
            let source_frame = clip_info.source_offset + (local_frame as f32 * speed) as u64;

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
                match &effect.kind {
                    EffectKind::ColorGrade {
                        brightness,
                        contrast,
                        saturation,
                        temperature,
                    } => {
                        let output = GpuBuffer::new(
                            &self.ctx,
                            frame_size,
                            vk::BufferUsageFlags::STORAGE_BUFFER,
                            MemoryLocation::GpuOnly,
                            "color_grade_out",
                        )?;

                        let push = ColorGradePush {
                            width,
                            height,
                            brightness: *brightness,
                            contrast: *contrast,
                            saturation: *saturation,
                            temperature: *temperature,
                        };

                        self.dispatch_2buffer(
                            &self.pipelines.color_grade,
                            &current_input,
                            &output,
                            bytemuck::bytes_of(&push),
                            width * height,
                        )?;

                        temp_buffers.push(current_input);
                        current_input = output;
                    }
                    EffectKind::Crop {
                        left,
                        top,
                        right,
                        bottom,
                    } => {
                        let crop_left = (*left * width as f32) as u32;
                        let crop_top = (*top * height as f32) as u32;
                        let crop_right = (*right * width as f32) as u32;
                        let crop_bottom = (*bottom * height as f32) as u32;
                        let dst_w = width.saturating_sub(crop_left + crop_right).max(1);
                        let dst_h = height.saturating_sub(crop_top + crop_bottom).max(1);
                        let dst_size = GpuBuffer::frame_buffer_size(dst_w, dst_h);

                        let output = GpuBuffer::new(
                            &self.ctx,
                            dst_size,
                            vk::BufferUsageFlags::STORAGE_BUFFER,
                            MemoryLocation::GpuOnly,
                            "crop_out",
                        )?;

                        let push = CropPush {
                            src_width: width,
                            src_height: height,
                            dst_width: dst_w,
                            dst_height: dst_h,
                            left: crop_left,
                            top: crop_top,
                            right: crop_right,
                            bottom: crop_bottom,
                        };

                        self.dispatch_2buffer(
                            &self.pipelines.crop,
                            &current_input,
                            &output,
                            bytemuck::bytes_of(&push),
                            dst_w * dst_h,
                        )?;

                        temp_buffers.push(current_input);
                        current_input = output;
                    }
                    // Skip non-video effects
                    EffectKind::Speed { .. }
                    | EffectKind::FadeIn { .. }
                    | EffectKind::FadeOut { .. }
                    | EffectKind::Volume { .. } => {}
                    EffectKind::Transition { .. } => {
                        // Transitions are handled separately between clips
                    }
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

    fn apply_transitions(
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

    /// Dispatch a compute shader with 2 storage buffer bindings.
    fn dispatch_2buffer(
        &self,
        pipeline: &crate::pipeline::ComputePipeline,
        input: &GpuBuffer,
        output: &GpuBuffer,
        push_constants: &[u8],
        pixel_count: u32,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();

        // Allocate descriptor set
        let set_layouts = [pipeline.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pipelines.descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)? }[0];

        // Update descriptor set
        let input_info = vk::DescriptorBufferInfo::default()
            .buffer(input.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let output_info = vk::DescriptorBufferInfo::default()
            .buffer(output.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let input_infos = [input_info];
        let output_infos = [output_info];

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&input_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&output_infos),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };

        self.submit_compute(pipeline, descriptor_set, push_constants, pixel_count)?;

        Ok(())
    }

    /// Dispatch a compute shader with 3 storage buffer bindings.
    fn dispatch_3buffer(
        &self,
        pipeline: &crate::pipeline::ComputePipeline,
        buf_a: &GpuBuffer,
        buf_b: &GpuBuffer,
        output: &GpuBuffer,
        push_constants: &[u8],
        pixel_count: u32,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();

        let set_layouts = [pipeline.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pipelines.descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)? }[0];

        let a_info = vk::DescriptorBufferInfo::default()
            .buffer(buf_a.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let b_info = vk::DescriptorBufferInfo::default()
            .buffer(buf_b.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let out_info = vk::DescriptorBufferInfo::default()
            .buffer(output.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let a_infos = [a_info];
        let b_infos = [b_info];
        let out_infos = [out_info];

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&a_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&b_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&out_infos),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };

        self.submit_compute(pipeline, descriptor_set, push_constants, pixel_count)?;

        Ok(())
    }

    /// Record and submit a compute dispatch, then wait for completion.
    fn submit_compute(
        &self,
        pipeline: &crate::pipeline::ComputePipeline,
        descriptor_set: vk::DescriptorSet,
        push_constants: &[u8],
        pixel_count: u32,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();

        unsafe {
            device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(self.command_buffer, &begin_info)?;

            device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.pipeline,
            );

            device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.layout,
                0,
                &[descriptor_set],
                &[],
            );

            device.cmd_push_constants(
                self.command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constants,
            );

            let group_count = pixel_count.div_ceil(256);
            device.cmd_dispatch(self.command_buffer, group_count, 1, 1);

            // Memory barrier for compute → compute/transfer
            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(
                    vk::AccessFlags::SHADER_READ
                        | vk::AccessFlags::TRANSFER_READ
                        | vk::AccessFlags::HOST_READ,
                );
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER
                    | vk::PipelineStageFlags::TRANSFER
                    | vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );

            device.end_command_buffer(self.command_buffer)?;

            device.reset_fences(&[self.fence])?;

            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
            device.queue_submit(self.ctx.compute_queue(), &[submit_info], self.fence)?;

            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }

        Ok(())
    }

    /// Clear a buffer to zero.
    fn clear_buffer(&self, buffer: &GpuBuffer, size: u64) -> Result<(), GpuError> {
        let device = self.ctx.device();
        unsafe {
            device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(self.command_buffer, &begin_info)?;
            device.cmd_fill_buffer(self.command_buffer, buffer.vk_buffer(), 0, size, 0);

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE);
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );

            device.end_command_buffer(self.command_buffer)?;
            device.reset_fences(&[self.fence])?;
            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
            device.queue_submit(self.ctx.compute_queue(), &[submit_info], self.fence)?;
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }
        Ok(())
    }

    /// Copy data between two buffers.
    fn copy_buffer(&self, src: &GpuBuffer, dst: &GpuBuffer, size: u64) -> Result<(), GpuError> {
        let device = self.ctx.device();
        unsafe {
            device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(self.command_buffer, &begin_info)?;

            let region = vk::BufferCopy::default().size(size);
            device.cmd_copy_buffer(
                self.command_buffer,
                src.vk_buffer(),
                dst.vk_buffer(),
                &[region],
            );

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ);
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );

            device.end_command_buffer(self.command_buffer)?;
            device.reset_fences(&[self.fence])?;
            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
            device.queue_submit(self.ctx.compute_queue(), &[submit_info], self.fence)?;
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }
        Ok(())
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

// --- Helper types and functions ---

struct ActiveClip {
    timeline_start: u64,
    source_offset: u64,
    opacity: f32,
    speed_factor: f32,
    media_path: Option<String>,
    effects: Vec<tazama_core::Effect>,
}

/// Collect video clips active at the given frame, ordered bottom-to-top.
/// Respects muted, visible, and solo flags.
fn collect_active_clips(timeline: &Timeline, frame_index: u64) -> Vec<ActiveClip> {
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
                let speed_factor = clip
                    .effects
                    .iter()
                    .find_map(|e| {
                        if let EffectKind::Speed { factor } = &e.kind {
                            Some(*factor)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(1.0);

                clips.push(ActiveClip {
                    timeline_start: clip.timeline_start,
                    source_offset: clip.source_offset,
                    opacity: clip.opacity,
                    speed_factor,
                    media_path: clip.media.as_ref().map(|m| m.path.clone()),
                    effects: clip.effects.clone(),
                });
            }
        }
    }

    clips
}

fn frame_index_to_ns(frame_index: u64, settings: &ProjectSettings) -> u64 {
    let fps = settings.frame_rate.numerator as f64 / settings.frame_rate.denominator as f64;
    (frame_index as f64 / fps * 1_000_000_000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
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
            }),
            timeline_start: 0,
            duration: 50,
            source_offset: 0,
            effects: vec![Effect {
                id: EffectId::new(),
                kind: EffectKind::Speed { factor: 2.0 },
                enabled: true,
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
}
