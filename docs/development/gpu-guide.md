# GPU Development Guide

## Overview

Tazama uses Vulkan compute shaders for video compositing and effects. All GPU code is in the `tazama-gpu` crate. No graphics pipeline, swapchain, or render passes — pure compute on flat RGBA storage buffers.

## Shader Compilation

Shaders are GLSL compute shaders in `crates/gpu/shaders/`. To compile:

```bash
make compile-shaders
# or directly:
./scripts/compile_shaders.sh
```

This requires `glslangValidator` (from the `glslang` or `vulkan-tools` package).

The compiled `.spv` files are embedded into the binary via `include_bytes!()` in `crates/gpu/src/shader.rs`. Unit tests validate that all 6 embedded shaders have correct SPIR-V magic numbers and 4-byte alignment.

### Current Shaders

| Shader | Push Constants | Purpose |
|--------|---------------|---------|
| `color_grade.comp` | brightness, contrast, saturation, temperature | Per-pixel color adjustment |
| `crop.comp` | src/dst dimensions, offsets | Region extraction |
| `composite.comp` | width, height, opacity | Alpha-over blending |
| `dissolve.comp` | width, height, progress | Cross-dissolve transition |
| `wipe.comp` | width, height, progress | Horizontal wipe transition |
| `fade.comp` | width, height, progress | Fade to/from black |

## Adding a New Effect

1. Write `crates/gpu/shaders/my_effect.comp` (use `layout(local_size_x = 256) in`)
2. Run `make compile-shaders`
3. Add `include_bytes!()` constant in `shader.rs`
4. Define a push constant struct in `pipeline.rs` (derive `bytemuck::Pod, Zeroable`)
5. Add pipeline to `PipelineCache::new()` via `create_pipeline()`
6. Add `EffectKind` variant in `crates/core/src/effect.rs`
7. Integrate dispatch in `render.rs` (match on `EffectKind`)
8. Add shader alignment/magic tests

## Buffer Lifecycle

```
Decode (CPU) → CpuToGpu staging → Compute effects → GpuOnly intermediates
                                                          ↓
                                        GpuToCpu readback → Export/Preview (CPU)
```

Buffers are created per-frame and destroyed after use. For a 1080p frame: `1920 * 1080 * 4 = ~8MB`.

The `GpuBuffer` type wraps Vulkan buffers with `gpu-allocator` memory management. The allocator is stored as `Option<Allocator>` in `GpuContext` to ensure it's dropped before the Vulkan device during cleanup.

## Crate Boundary Traits

The GPU crate defines two traits to avoid depending on the media crate:

- **`FrameSource`** — Provides decoded RGBA frames to the renderer. Implemented by `MediaFrameSource` in the app crate, which bridges to GStreamer's video decoder.
- **`AudioOutput`** — Controls audio playback state during preview. Implemented by the app layer wrapping `tazama-media::AudioPreview`.

## Architecture

```
Renderer
├── GpuContext (instance, device, queue, allocator)
├── PipelineCache (6 compute pipelines)
├── CommandBuffer + Fence (dispatch synchronization)
└── GpuBuffer (staging/compute/readback buffers)

PreviewLoop → renders at project FPS, drops frames if behind
ExportRender → renders every frame sequentially for export
```

### Rendering Pipeline

For each frame, the renderer:
1. Collects active video clips (respects mute/solo/visible)
2. Decodes source frame via `FrameSource`
3. Uploads to GPU staging buffer
4. Applies per-clip effects (ColorGrade → Crop, skips audio effects)
5. Composites onto accumulator with clip opacity
6. Applies transitions between adjacent clips
7. Reads back final RGBA frame from GPU

## Testing

### Unit tests (no GPU required)

```bash
cargo test -p tazama-gpu
```

Tests cover: clip collection logic, frame indexing math, speed factor extraction, muted/solo/invisible track exclusion, buffer sizing, SPIR-V shader validation.

### With Vulkan

With a real GPU:
```bash
cargo test -p tazama-gpu --features gpu-tests
```

With lavapipe (software Vulkan):
```bash
# Install: pacman -S vulkan-swrast (Arch) or apt install mesa-vulkan-drivers (Debian)
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json cargo test -p tazama-gpu --features gpu-tests
```

## Vulkan Debugging

Enable validation layers:
```bash
VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation cargo run
```

Check which Vulkan devices are available:
```bash
vulkaninfo --summary
```
