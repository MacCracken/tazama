# Tazama Testing Matrix

## Current Status (2026.3.19)

- **686 tests** across 6 crates
- **51.6% line coverage** (1790/3468 lines)
- **Zero clippy warnings** (`cargo clippy --workspace --all-targets -- -D warnings`)
- Benchmark suite: 11 criterion benchmarks (DSP, keyframes, timeline serde)

## Test Distribution

| Crate | Tests | Coverage | Notes |
|-------|-------|----------|-------|
| **tazama-core** | 199 | ~85% | Keyframes, commands, timeline, effects, clips |
| **tazama-media** | 286 | ~45% | DSP (97%), mix (55%), probe (82%), export (13%), decode (0%) |
| **tazama-gpu** | 60 | ~15% | LUT (100%), text (100%), collect (97%), render/dispatch (0%) |
| **tazama-storage** | 33 | ~88% | DB (93%), project (100%), media (94%), autosave (60%) |
| **tazama-mcp** | 81 | ~62% | Unit tests + integration test |
| **tazama (app)** | 27 | ~30% | IPC commands, limited without full app context |

## Hardware Requirements

### Required (all tests)
- **Rust** 1.85+ (edition 2024, MSRV 1.89 recommended)
- **GStreamer** 1.20+ with plugins: `gst-plugins-base`, `gst-plugins-good`, `gst-plugins-bad`
- **pkg-config** (for GStreamer/ALSA/PipeWire detection)

### Required (full functionality)
- **PipeWire** or **ALSA** (audio preview/recording — CPAL backend)
- **Vulkan** ICD loader + driver (GPU rendering — `vulkan-icd-loader`)
- **SQLite** (project storage)

### Optional
- **VAAPI** headers + driver (hardware video encode testing)
- **NVENC** (NVIDIA GPU encode testing)

## Reference Development Machine

| Component | Details |
|-----------|---------|
| CPU | AMD Ryzen 7 5800H (8C/16T, Zen 3) |
| RAM | 60 GB DDR4 |
| GPU | AMD Radeon Vega (Cezanne, integrated) — Vulkan 1.3 via RADV |
| OS | Arch Linux, kernel 6.12.71-1-lts |
| Rust | 1.93.0 |
| GStreamer | 1.28.1 |
| PipeWire | 1.4.10 |
| ALSA | 1.2.15.3 |

## Coverage by Module

### High Coverage (85%+)

| Module | Coverage | Key Tests |
|--------|----------|-----------|
| DSP (compressor) | 97% | NaN guards, silence, stereo, envelope dynamics |
| DSP (EQ) | 96% | Coefficient validation, per-band boost/cut, multichannel |
| DSP (noise reduction) | 100% | Spectral gating, short buffer fallback, NaN guards |
| DSP (reverb) | 100% | Comb/allpass filters, room size, stereo, param clamping |
| GPU text.rs | 100% | Text rasterization, dimensions |
| GPU lut.rs | 100% | .cube parsing, trilinear interpolation |
| GPU collect.rs | 97% | Clip collection, solo/mute/visible, speed extraction |
| Storage db.rs | 93% | Cache round-trip, project CRUD, JSON size limits |
| Storage media.rs | 94% | Import, overwrite, directory creation |
| Storage project.rs | 100% | Save/load round-trip, error handling |
| Core timeline.rs | 98% | Overlap detection, markers, solo/visible, duration |
| Core command.rs | ~90% | All 16 variants symmetric apply/undo, 34 tests |
| Core keyframe.rs | ~90% | Bezier extremes, same-frame, integrated speed |

### Medium Coverage (40-84%)

| Module | Coverage | Gap Reason |
|--------|----------|------------|
| probe.rs | 82% | GStreamer success path needs real media |
| export/mod.rs | 68% | Encoder serde covered, pipeline needs GStreamer |
| MCP main.rs | 62% | Handler unit tests, some branches need full server |
| mix.rs | 55% | Fade/pan covered, full mix needs decoded audio |
| autosave.rs | 60% | Core logic covered, background loop hard to test |
| record.rs | 49% | WAV header covered, CPAL callback needs audio device |
| proxy.rs | 48% | Extension checks covered, GStreamer pipeline needs media |

### Low Coverage (0-39%)

| Module | Coverage | Gap Reason |
|--------|----------|------------|
| export/pipeline.rs | 13% | Full encode pipeline needs GStreamer + media |
| thumbnail.rs | 22% | Spec tests covered, frame extraction needs media |
| waveform.rs | 8% | Peak math covered, extraction needs GStreamer |
| decode/audio.rs | 0% | Requires real audio files + GStreamer |
| decode/video.rs | 0% | Requires real video files + GStreamer |
| playback.rs | 0% | Requires CPAL audio device |
| GPU render/* | 0% | Requires Vulkan GPU context |
| GPU pipeline.rs | 0% | Requires Vulkan GPU context |

## Running Tests

```bash
# Full test suite
cargo test --workspace

# Specific crate
cargo test -p tazama-core
cargo test -p tazama-media

# GPU integration tests (requires Vulkan — AMD Radeon Vega via RADV on dev machine)
cargo test -p tazama-gpu

# With tarang feature
cargo test -p tazama-media --features tarang

# Coverage report
cargo tarpaulin --workspace --skip-clean --out html

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Benchmarks
cargo bench --workspace
```

### GPU Integration Tests

The `tazama-gpu` crate includes integration tests in `crates/gpu/tests/gpu_integration.rs`
that create real Vulkan contexts and dispatch compute shaders on the GPU. These tests
require a Vulkan-capable device and driver. On the reference development machine they
run against the AMD Radeon Vega (Cezanne) integrated GPU using the RADV Mesa driver
(Vulkan 1.4).

Tests covered: context creation, pipeline cache compilation (8 shaders), renderer
creation, GPU buffer write/read roundtrip, empty timeline rendering, color grade
effect, crop effect, and frame buffer size calculations.

## CI Requirements

For CI environments that need to run the full test suite:

```bash
# Arch Linux / AGNOS
pacman -S gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad \
          pipewire alsa-lib vulkan-icd-loader sqlite pkg-config \
          dav1d libvpx openh264 opus libfdk-aac

# Ubuntu/Debian
apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
                gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
                libpipewire-0.3-dev libasound2-dev libvulkan-dev \
                libsqlite3-dev pkg-config \
                libdav1d-dev libvpx-dev libopenh264-dev libopus-dev libfdk-aac-dev
```

## Improving Coverage

The remaining untested code falls into three categories:

1. **GStreamer pipelines** (decode, export, waveform, thumbnail) — need real media fixture files or a GStreamer mock/stub layer
2. **GPU rendering** (render, dispatch, transitions, pipeline) — need a Vulkan mock context or headless GPU (lavapipe)
3. **Audio hardware** (playback, record) — need CPAL audio device mock or virtual audio device (e.g., `snd-aloop`)
