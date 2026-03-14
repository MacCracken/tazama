# GPU Development Guide

## Shader Compilation

Shaders are GLSL compute shaders in `crates/gpu/shaders/`. To compile:

```bash
make compile-shaders
# or directly:
./scripts/compile_shaders.sh
```

This requires `glslangValidator` (from the `glslang` or `vulkan-tools` package).

The compiled `.spv` files are embedded into the binary via `include_bytes!()` in `crates/gpu/src/shader.rs`.

## Adding a New Effect

1. Write `crates/gpu/shaders/my_effect.comp` (use `layout(local_size_x = 256) in`)
2. Run `make compile-shaders`
3. Add `include_bytes!()` constant in `shader.rs`
4. Define a push constant struct in `pipeline.rs` (derive `bytemuck::Pod`)
5. Add pipeline to `PipelineCache::new()` via `create_pipeline()`
6. Integrate dispatch in `render.rs` (match on `EffectKind`)

## Buffer Lifecycle

```
Decode (CPU) → CpuToGpu staging → Compute effects → GpuOnly intermediates
                                                          ↓
                                        GpuToCpu readback → Export/Preview (CPU)
```

Buffers are created per-frame and destroyed after use. For a 1080p frame: `1920 * 1080 * 4 = ~8MB`.

## Testing

### Unit tests (no GPU required)

```bash
cargo test -p tazama-gpu
```

Tests clip collection, frame indexing, speed factor logic, and buffer sizing.

### GPU integration tests (requires Vulkan)

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
