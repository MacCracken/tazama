use ash::vk;
use bytemuck;
use gpu_allocator::MemoryLocation;
use tazama_core::{Effect, EffectKind, keyframe};

use crate::buffer::GpuBuffer;
use crate::context::GpuError;
use crate::lut;
use crate::pipeline::{ColorGradePush, CropPush, LutPush, PipTransformPush};
use crate::text;

use super::Renderer;

/// Resolve a keyframed parameter: evaluate at the current frame and override the static value.
pub(crate) fn resolve_param(effect: &Effect, frame_index: u64, name: &str, static_val: f32) -> f32 {
    for kt in &effect.keyframe_tracks {
        if kt.parameter == name {
            if let Some(v) = keyframe::evaluate(kt, frame_index) {
                return v;
            }
        }
    }
    static_val
}

/// Bundles the read-only context needed by [`apply_effect`].
#[allow(dead_code)]
pub(crate) struct EffectContext<'a> {
    pub renderer: &'a Renderer,
    pub effect: &'a Effect,
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub input_buffer: &'a GpuBuffer,
    pub output_buffer: &'a GpuBuffer,
    pub extra_buffer: Option<&'a GpuBuffer>,
    pub pixel_count: u32,
}

/// Apply a single effect described by `ectx`.
///
/// Uses `ectx.input_buffer` as the source. Returns `Some(output)` when a new
/// buffer was produced (the caller is responsible for retiring the old input
/// into `temp_buffers`), or `None` when the effect is a no-op passthrough.
pub(crate) fn apply_effect(ectx: &EffectContext) -> Result<Option<GpuBuffer>, GpuError> {
    let renderer = ectx.renderer;
    let ctx = &renderer.ctx;
    let pipelines = &renderer.pipelines;
    let effect = ectx.effect;
    let frame_index = ectx.frame_index;
    let width = ectx.width;
    let height = ectx.height;
    let pixel_count = ectx.pixel_count;
    let input = ectx.input_buffer;
    let frame_size = GpuBuffer::frame_buffer_size(width, height);

    match &effect.kind {
        EffectKind::ColorGrade {
            brightness,
            contrast,
            saturation,
            temperature,
        } => {
            let brightness = resolve_param(effect, frame_index, "brightness", *brightness);
            let contrast = resolve_param(effect, frame_index, "contrast", *contrast);
            let saturation = resolve_param(effect, frame_index, "saturation", *saturation);
            let temperature = resolve_param(effect, frame_index, "temperature", *temperature);
            let output = GpuBuffer::new(
                ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::GpuOnly,
                "color_grade_out",
            )?;

            let push = ColorGradePush {
                width,
                height,
                brightness,
                contrast,
                saturation,
                temperature,
            };

            renderer.dispatch_2buffer(
                &pipelines.color_grade,
                input,
                &output,
                bytemuck::bytes_of(&push),
                pixel_count,
            )?;

            Ok(Some(output))
        }
        EffectKind::Crop {
            left,
            top,
            right,
            bottom,
        } => {
            let left = resolve_param(effect, frame_index, "left", *left).clamp(0.0, 1.0);
            let top = resolve_param(effect, frame_index, "top", *top).clamp(0.0, 1.0);
            let right = resolve_param(effect, frame_index, "right", *right).clamp(0.0, 1.0);
            let bottom = resolve_param(effect, frame_index, "bottom", *bottom).clamp(0.0, 1.0);
            let crop_left = (left * width as f32) as u32;
            let crop_top = (top * height as f32) as u32;
            let crop_right = (right * width as f32) as u32;
            let crop_bottom = (bottom * height as f32) as u32;
            let dst_w = width
                .saturating_sub(crop_left.saturating_add(crop_right))
                .max(1);
            let dst_h = height
                .saturating_sub(crop_top.saturating_add(crop_bottom))
                .max(1);
            let dst_size = GpuBuffer::frame_buffer_size(dst_w, dst_h);

            let output = GpuBuffer::new(
                ctx,
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

            renderer.dispatch_2buffer(
                &pipelines.crop,
                input,
                &output,
                bytemuck::bytes_of(&push),
                dst_w * dst_h,
            )?;

            Ok(Some(output))
        }
        // Skip non-video effects and effects handled elsewhere
        EffectKind::Speed { .. }
        | EffectKind::FadeIn { .. }
        | EffectKind::FadeOut { .. }
        | EffectKind::Volume { .. }
        | EffectKind::Eq { .. }
        | EffectKind::Compressor { .. }
        | EffectKind::NoiseReduction { .. }
        | EffectKind::Reverb { .. }
        | EffectKind::Plugin { .. } => Ok(None),
        EffectKind::Lut { lut_path } => {
            // Parse the .cube LUT file
            let lut_content = std::fs::read_to_string(lut_path)
                .map_err(|e| GpuError::Other(format!("failed to read LUT file: {e}")))?;
            let lut_data = lut::parse_cube(&lut_content)
                .map_err(|e| GpuError::Other(format!("failed to parse LUT: {e}")))?;

            // Upload LUT data as vec4 array (pad RGB to RGBA)
            let lut_floats: Vec<f32> = lut_data
                .data
                .iter()
                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 0.0])
                .collect();
            let lut_bytes = bytemuck::cast_slice::<f32, u8>(&lut_floats);
            let lut_buf_size = lut_bytes.len() as u64;

            let mut lut_buffer = GpuBuffer::new(
                ctx,
                lut_buf_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::CpuToGpu,
                "lut_data",
            )?;
            lut_buffer.write(lut_bytes)?;

            let output = GpuBuffer::new(
                ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::GpuOnly,
                "lut_out",
            )?;

            let push = LutPush {
                width,
                height,
                lut_size: lut_data.size,
                _pad: 0,
            };

            renderer.dispatch_3buffer(
                &pipelines.lut,
                input,
                &output,
                &lut_buffer,
                bytemuck::bytes_of(&push),
                pixel_count,
            )?;

            lut_buffer.destroy(ctx);
            Ok(Some(output))
        }
        EffectKind::Transform {
            scale_x,
            scale_y,
            translate_x,
            translate_y,
        } => {
            // For Transform, we treat the current frame as the overlay
            // and composite it onto a transparent base at the given
            // position and scale. The base is the same size as the output.
            let output = GpuBuffer::new(
                ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::GpuOnly,
                "transform_out",
            )?;

            // Create a transparent base buffer
            let base = GpuBuffer::new(
                ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                MemoryLocation::GpuOnly,
                "transform_base",
            )?;
            renderer.clear_buffer(&base, frame_size)?;

            let push = PipTransformPush {
                base_width: width,
                base_height: height,
                overlay_width: width,
                overlay_height: height,
                scale_x: *scale_x,
                scale_y: *scale_y,
                translate_x: *translate_x,
                translate_y: *translate_y,
            };

            renderer.dispatch_3buffer(
                &pipelines.transform,
                &base,
                input,
                &output,
                bytemuck::bytes_of(&push),
                pixel_count,
            )?;

            base.destroy(ctx);
            Ok(Some(output))
        }
        EffectKind::Text {
            content,
            font_family,
            font_size: fsize,
            color,
            x,
            y,
        } => {
            // Rasterize text to an RGBA buffer
            let max_w = width;
            let max_h = height;
            let (text_rgba, text_w, text_h) =
                text::rasterize_text(content, font_family, *fsize, *color, max_w, max_h);

            // Upload text overlay
            let text_buf_size = GpuBuffer::frame_buffer_size(text_w, text_h);
            let mut text_buffer = GpuBuffer::new(
                ctx,
                text_buf_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::CpuToGpu,
                "text_overlay",
            )?;
            text_buffer.write(&text_rgba)?;

            let output = GpuBuffer::new(
                ctx,
                frame_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::GpuOnly,
                "text_out",
            )?;

            // Use transform pipeline to composite text at (x, y)
            let push = PipTransformPush {
                base_width: width,
                base_height: height,
                overlay_width: text_w,
                overlay_height: text_h,
                scale_x: 1.0,
                scale_y: 1.0,
                translate_x: *x,
                translate_y: *y,
            };

            renderer.dispatch_3buffer(
                &pipelines.transform,
                input,
                &text_buffer,
                &output,
                bytemuck::bytes_of(&push),
                pixel_count,
            )?;

            text_buffer.destroy(ctx);
            Ok(Some(output))
        }
        EffectKind::Transition { .. } => {
            // Transitions are handled separately between clips
            Ok(None)
        }
    }
}
