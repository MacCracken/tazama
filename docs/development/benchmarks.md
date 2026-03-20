# Tazama Benchmarks

## Reference Hardware

| Component | Details |
|-----------|---------|
| **CPU** | AMD Ryzen 7 5800H (8 cores / 16 threads, Zen 3) |
| **RAM** | 60 GB DDR4 |
| **GPU** | AMD Radeon Vega (Cezanne, integrated) |
| **OS** | Arch Linux, kernel 6.12.71-1-lts |
| **Rust** | 1.93.0 (edition 2024) |
| **GStreamer** | 1.28.1 (base + good + bad plugins) |

## Benchmark Results (2026.3.19)

All benchmarks run via `cargo bench --workspace` using criterion 0.5 with 100 samples each.

### DSP Processing (1 second stereo 48kHz buffer = 96,000 samples)

| Effect | Time | Throughput |
|--------|------|------------|
| **3-band EQ** | 299 µs | ~320× real-time |
| **Compressor** | 459 µs | ~208× real-time |
| **Reverb** (Schroeder) | 860 µs | ~111× real-time |
| **Noise Reduction** (STFT spectral gate) | 1.43 ms | ~67× real-time |

All DSP effects process well above real-time on a single core. Noise reduction is the most expensive due to FFT (2048-point) + overlap-add.

### Keyframe Evaluation

| Keyframes | evaluate() | Notes |
|-----------|-----------|-------|
| **10** | 8.1 ns | O(log n) binary search |
| **100** | 12.7 ns | ~1.6× slower than 10 |
| **1000** | 16.7 ns | ~2.1× slower than 10 |

Keyframe lookup scales sub-linearly — binary search dominates. Even with 1000 keyframes per parameter, evaluation is negligible.

### Integrated Speed (variable speed accumulation)

| Track Length | Time | Per-frame |
|-------------|------|-----------|
| **100 frames** | 1.89 µs | 18.9 ns/frame |
| **500 frames** | 4.81 µs | 9.6 ns/frame |
| **2000 frames** | 10.7 µs | 5.4 ns/frame |

Linear scan over keyframe segments — amortized cost decreases with longer tracks.

### Timeline Serialization (10 tracks × 100 clips)

| Operation | Time | Size |
|-----------|------|------|
| **Serialize** (JSON) | 231 µs | ~large project |
| **Deserialize** (JSON) | 412 µs | — |

Autosave at 30-second intervals with 231 µs serialization adds zero perceptible overhead.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark group
cargo bench -p tazama-media --bench dsp
cargo bench -p tazama-core --bench keyframe

# Run a specific benchmark
cargo bench -p tazama-core --bench keyframe -- "keyframe_evaluate/100"
```

HTML reports are generated in `target/criterion/` (excluded from git).

## Planned Benchmarks

- GPU render — frame render time at 1080p/4K with effect chains
- Export pipeline — encode throughput per format
- Probe/decode — media file probe latency, video decode frame rate
- Tarang vs GStreamer — comparative decode/probe benchmarks
