# ADR 002: GPU Compute Pipeline Architecture

## Status

Accepted

## Context

Tazama needs real-time video compositing and effects processing. The renderer must composite multi-track timelines, apply per-clip effects (color grading, crop), handle transitions between clips (dissolve, wipe, fade), and produce RGBA frames for both preview and export.

## Decision

### Compute-only Vulkan

Use Vulkan compute shaders exclusively — no graphics pipeline, no swapchain, no render passes. All operations work on flat storage buffers of packed RGBA pixels (`u32` per pixel). This simplifies the Vulkan setup significantly while being sufficient for video compositing.

### Pre-compiled SPIR-V

GLSL compute shaders are compiled to SPIR-V offline using `glslangValidator`. The `.spv` files are checked into the repo and loaded at build time via `include_bytes!()`. No runtime shader compilation dependencies.

### One pipeline per effect

Each effect (color_grade, crop, composite, dissolve, wipe, fade) has its own compute pipeline with a dedicated push constant layout. This is simpler to debug and profile than an uber-shader approach.

### Buffer strategy

Three memory locations:
- **CpuToGpu** — Staging buffers for uploading decoded frames from CPU
- **GpuOnly** — Intermediate compute buffers (accumulator, effect outputs)
- **GpuToCpu** — Readback buffers for downloading rendered frames

Each 1080p RGBA frame is ~8MB. Memory footprint is trivial.

### Software fallback

Vulkan is loaded at runtime via `Entry::load()`. On systems without a GPU, lavapipe (software Vulkan) works transparently by setting `VK_ICD_FILENAMES`. The selected device name is logged at init.

### GpuFrame type

Decoded frames for GPU processing use a `GpuFrame` struct in tazama-gpu (not `VideoFrame` from tazama-media). A `FrameSource` trait decouples the renderer from the media decoder.

## Consequences

- All GPU code is self-contained in the `tazama-gpu` crate
- Adding new effects requires: write `.comp` shader, compile to `.spv`, add push constant struct, add pipeline to `PipelineCache`, integrate in renderer
- Software rendering via lavapipe is available but slow; real GPU recommended for production
- No runtime shader compilation — faster startup, smaller dependency tree
