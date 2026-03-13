# ADR-001: Rust + Vulkan + GStreamer

## Status
Accepted

## Context
Tazama is an AI-native video editor targeting the AGNOS desktop. We need a media processing stack that is performant, GPU-accelerated, and integrates well with the existing AGNOS infrastructure (GStreamer, Vulkan, PipeWire already in desktop recipes).

## Decision
- **Rust** for all application code (consistent with AGNOS ecosystem)
- **GStreamer** (`gstreamer-rs`) for media decode/encode/pipeline management
- **Vulkan** (`ash`) for GPU compute — effects, color grading, compositing, rendering
- **PipeWire** for audio routing and monitoring
- **Tauri v2** for the desktop shell (React/TypeScript frontend)

## Consequences
- Native performance for real-time preview and export
- GStreamer provides codec coverage without reimplementing demuxers/decoders
- Vulkan compute shaders enable parallel frame processing on GPU
- Ash (raw Vulkan bindings) gives full control vs higher-level abstractions
- Requires Vulkan-capable GPU; software fallback via lavapipe for development
