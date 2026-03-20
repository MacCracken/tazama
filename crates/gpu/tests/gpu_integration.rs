//! GPU integration tests — require a real Vulkan GPU (AMD Radeon Vega via RADV).
//!
//! These tests create actual Vulkan contexts and execute compute shaders on the
//! GPU. They will fail on machines without a Vulkan-capable device.

use std::sync::Arc;

use ash::vk;
use bytes::Bytes;
use gpu_allocator::MemoryLocation;

use tazama_core::*;
use tazama_gpu::buffer::GpuBuffer;
use tazama_gpu::*;

// ---------------------------------------------------------------------------
// Mock FrameSource
// ---------------------------------------------------------------------------

/// A frame source that always returns `None`-equivalent (error) for any path.
struct NullFrameSource;

impl FrameSource for NullFrameSource {
    fn get_frame(&self, _media_path: &str, _frame_index: u64) -> Result<GpuFrame, GpuError> {
        Err(GpuError::FrameSource("null frame source".into()))
    }
}

/// A frame source that returns a solid-color RGBA frame of the requested
/// dimensions. Width/height are fixed at construction time.
struct SolidFrameSource {
    width: u32,
    height: u32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl FrameSource for SolidFrameSource {
    fn get_frame(&self, _media_path: &str, frame_index: u64) -> Result<GpuFrame, GpuError> {
        let pixel_count = (self.width * self.height) as usize;
        let mut data = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            data.push(self.r);
            data.push(self.g);
            data.push(self.b);
            data.push(self.a);
        }
        Ok(GpuFrame {
            frame_index,
            width: self.width,
            height: self.height,
            data: Bytes::from(data),
            timestamp_ns: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn small_settings(width: u32, height: u32) -> ProjectSettings {
    ProjectSettings {
        width,
        height,
        frame_rate: FrameRate {
            numerator: 30,
            denominator: 1,
        },
        sample_rate: 48000,
        channels: 2,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn gpu_context_creation() {
    let ctx = GpuContext::new();
    assert!(ctx.is_ok(), "GPU context should initialize on this machine");
}

#[test]
fn pipeline_cache_creation() {
    let ctx = Arc::new(GpuContext::new().unwrap());
    let cache = PipelineCache::new(&ctx);
    assert!(cache.is_ok(), "Pipeline cache should compile all 8 shaders");
    // Explicitly destroy to avoid leak
    cache.unwrap().destroy(ctx.device());
}

#[test]
fn renderer_creation() {
    let ctx = Arc::new(GpuContext::new().unwrap());
    let renderer = Renderer::new(ctx);
    assert!(renderer.is_ok());
}

#[test]
fn buffer_write_read_roundtrip() {
    let ctx = Arc::new(GpuContext::new().unwrap());

    let data: Vec<u8> = (0..=255).collect();
    let mut buf = GpuBuffer::new(
        &ctx,
        256,
        vk::BufferUsageFlags::STORAGE_BUFFER,
        MemoryLocation::CpuToGpu,
        "test_roundtrip",
    )
    .unwrap();

    buf.write(&data).unwrap();
    let readback = buf.read(256).unwrap();
    assert_eq!(readback, &data[..]);

    buf.destroy(&ctx);
}

#[test]
fn render_empty_timeline() {
    let ctx = Arc::new(GpuContext::new().unwrap());
    let renderer = Renderer::new(ctx).unwrap();
    let timeline = Timeline::new();
    let settings = small_settings(256, 256);

    let result = renderer.render_frame(&timeline, 0, &NullFrameSource, &settings);
    assert!(result.is_ok());

    let frame = result.unwrap();
    assert_eq!(frame.width, 256);
    assert_eq!(frame.height, 256);
    // Empty timeline produces transparent black (all zeroes).
    assert!(frame.data.iter().all(|&b| b == 0));
}

#[test]
fn render_clip_with_color_grade() {
    let ctx = Arc::new(GpuContext::new().unwrap());
    let renderer = Renderer::new(ctx).unwrap();
    let settings = small_settings(64, 64);

    let source = SolidFrameSource {
        width: 64,
        height: 64,
        r: 128,
        g: 0,
        b: 0,
        a: 255,
    };

    let mut timeline = Timeline::new();
    timeline.add_track(Track::new("V1", TrackKind::Video));

    let clip = Clip {
        id: ClipId::new(),
        name: "solid_red".to_string(),
        kind: ClipKind::Video,
        media: Some(MediaRef {
            path: "/virtual/red.mp4".to_string(),
            duration_frames: 100,
            width: Some(64),
            height: Some(64),
            sample_rate: None,
            channels: None,
            info: None,
            proxy_path: None,
        }),
        timeline_start: 0,
        duration: 100,
        source_offset: 0,
        effects: vec![Effect {
            id: EffectId::new(),
            kind: EffectKind::ColorGrade {
                brightness: 0.5,
                contrast: 0.0,
                saturation: 0.0,
                temperature: 0.0,
            },
            enabled: true,
            keyframe_tracks: vec![],
        }],
        opacity: 1.0,
        volume: 1.0,
    };
    timeline.tracks[0].add_clip(clip).unwrap();

    let result = renderer.render_frame(&timeline, 0, &source, &settings);
    assert!(result.is_ok(), "render_frame failed: {:?}", result.err());

    let frame = result.unwrap();
    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 64);

    // The brightness=0.5 should have modified the red channel.
    // Check that output is not identical to the input (solid 128,0,0,255).
    let pixel_count = 64 * 64;
    let mut all_unchanged = true;
    for i in 0..pixel_count {
        let offset = i * 4;
        let r = frame.data[offset];
        if r != 128 {
            all_unchanged = false;
            break;
        }
    }
    assert!(
        !all_unchanged,
        "Color grade with brightness=0.5 should modify pixel values"
    );
}

#[test]
fn render_clip_with_crop() {
    let ctx = Arc::new(GpuContext::new().unwrap());
    let renderer = Renderer::new(ctx).unwrap();
    let settings = small_settings(64, 64);

    let source = SolidFrameSource {
        width: 64,
        height: 64,
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };

    let mut timeline = Timeline::new();
    timeline.add_track(Track::new("V1", TrackKind::Video));

    // Crop 25% from each side => output is 50% of input in each dimension.
    let clip = Clip {
        id: ClipId::new(),
        name: "solid_green".to_string(),
        kind: ClipKind::Video,
        media: Some(MediaRef {
            path: "/virtual/green.mp4".to_string(),
            duration_frames: 100,
            width: Some(64),
            height: Some(64),
            sample_rate: None,
            channels: None,
            info: None,
            proxy_path: None,
        }),
        timeline_start: 0,
        duration: 100,
        source_offset: 0,
        effects: vec![Effect {
            id: EffectId::new(),
            kind: EffectKind::Crop {
                left: 0.25,
                top: 0.25,
                right: 0.25,
                bottom: 0.25,
            },
            enabled: true,
            keyframe_tracks: vec![],
        }],
        opacity: 1.0,
        volume: 1.0,
    };
    timeline.tracks[0].add_clip(clip).unwrap();

    let result = renderer.render_frame(&timeline, 0, &source, &settings);
    assert!(result.is_ok(), "render_frame failed: {:?}", result.err());

    let frame = result.unwrap();
    // The crop effect produces a smaller buffer internally, but the composite
    // step writes into the full-size accumulator. Verify the frame renders
    // without error and has the project dimensions.
    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 64);
}

#[test]
fn frame_buffer_size_matches_allocation() {
    let size = GpuBuffer::frame_buffer_size(1920, 1080);
    assert_eq!(size, 1920 * 1080 * 4);
}
