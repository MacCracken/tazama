//! Comparative benchmark: Tarang (symphonia) vs GStreamer for media probing
//! and audio decoding.
//!
//! Run with:
//!   cargo bench -p tazama-media --bench tarang_vs_gstreamer

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;

/// Build a valid WAV file (PCM 16-bit, mono) in memory.
/// Mirrors the `make_wav_bytes` helper used in probe.rs tests.
fn make_wav_bytes(sample_rate: u32, duration_secs: u32) -> Vec<u8> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let num_samples = sample_rate * duration_secs;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = num_samples * num_channels as u32 * bits_per_sample as u32 / 8;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + data_size as usize);
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt sub-chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    // Generate a 440 Hz sine tone so it's not pure silence
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
        let pcm = (sample * i16::MAX as f64) as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }
    buf
}

/// Write a WAV file to disk at the given path.
fn write_test_wav(path: &Path, sample_rate: u32, duration_secs: u32) {
    let data = make_wav_bytes(sample_rate, duration_secs);
    std::fs::write(path, data).expect("failed to write test WAV");
}

// ---------------------------------------------------------------------------
// Probe benchmark
// ---------------------------------------------------------------------------

fn bench_probe(c: &mut Criterion) {
    tazama_media::init().ok();

    let dir = std::env::temp_dir().join("tazama_bench_probe");
    std::fs::create_dir_all(&dir).unwrap();

    // Create a 1-second 48 kHz WAV file
    let wav_path = dir.join("bench.wav");
    write_test_wav(&wav_path, 48000, 1);

    let mut group = c.benchmark_group("probe");

    // GStreamer probe: copy the WAV as .mxf so the tarang extension check
    // won't match and GStreamer's content-based detection is used instead.
    let gst_path = dir.join("bench_gst.mxf");
    std::fs::copy(&wav_path, &gst_path).unwrap();

    group.bench_function("gstreamer", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&gst_path)))
        })
    });

    // Tarang audio probe: .wav extension triggers the symphonia path
    group.bench_function("tarang_audio", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&wav_path)))
        })
    });

    group.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Audio decode benchmark
// ---------------------------------------------------------------------------

fn bench_audio_decode(c: &mut Criterion) {
    tazama_media::init().ok();

    let dir = std::env::temp_dir().join("tazama_bench_decode");
    std::fs::create_dir_all(&dir).unwrap();

    // 1-second 48 kHz WAV
    let wav_path = dir.join("bench_decode.wav");
    write_test_wav(&wav_path, 48000, 1);

    let mut group = c.benchmark_group("audio_decode");

    // GStreamer decode: use .mxf extension to bypass tarang
    let gst_path = dir.join("bench_decode_gst.mxf");
    std::fs::copy(&wav_path, &gst_path).unwrap();

    group.bench_function("gstreamer", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let mut rx =
                    tazama_media::decode::audio::AudioDecoder::decode(black_box(&gst_path))
                        .unwrap();
                let mut count = 0usize;
                while let Some(buf) = rx.recv().await {
                    count += buf.samples.len();
                }
                count
            })
        })
    });

    // Tarang decode: .wav extension triggers symphonia path
    group.bench_function("tarang", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let mut rx =
                    tazama_media::decode::audio::AudioDecoder::decode(black_box(&wav_path))
                        .unwrap();
                let mut count = 0usize;
                while let Some(buf) = rx.recv().await {
                    count += buf.samples.len();
                }
                count
            })
        })
    });

    group.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Video probe benchmark (real media fixtures)
// ---------------------------------------------------------------------------

fn bench_video_probe(c: &mut Criterion) {
    tazama_media::init().ok();

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    if !fixtures.join("test_h264.mp4").exists() {
        eprintln!("SKIP video benchmarks: run scripts/generate-test-fixtures.sh first");
        return;
    }

    let mut group = c.benchmark_group("video_probe");

    // MP4 (H.264) — tarang path (.mp4 extension)
    let mp4_path = fixtures.join("test_h264.mp4");
    group.bench_function("tarang_mp4", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&mp4_path)))
        })
    });

    // MP4 via GStreamer — copy to .mxf to bypass tarang extension check
    let gst_mp4 = fixtures.join("test_h264_gst.mxf");
    std::fs::copy(&mp4_path, &gst_mp4).ok();
    group.bench_function("gstreamer_mp4", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&gst_mp4)))
        })
    });

    // WebM (VP9) — tarang path
    let webm_path = fixtures.join("test_vp9.webm");
    group.bench_function("tarang_webm", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&webm_path)))
        })
    });

    // WebM via GStreamer
    let gst_webm = fixtures.join("test_vp9_gst.mxf");
    std::fs::copy(&webm_path, &gst_webm).ok();
    group.bench_function("gstreamer_webm", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&gst_webm)))
        })
    });

    // MKV (H.264) — tarang path
    let mkv_path = fixtures.join("test_h264.mkv");
    group.bench_function("tarang_mkv", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&mkv_path)))
        })
    });

    // MKV via GStreamer
    let gst_mkv = fixtures.join("test_h264_gst_mkv.mxf");
    std::fs::copy(&mkv_path, &gst_mkv).ok();
    group.bench_function("gstreamer_mkv", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::probe::probe(black_box(&gst_mkv)))
        })
    });

    group.finish();
    // Clean up GStreamer copies
    let _ = std::fs::remove_file(&gst_mp4);
    let _ = std::fs::remove_file(&gst_webm);
    let _ = std::fs::remove_file(&gst_mkv);
}

// ---------------------------------------------------------------------------
// Video decode benchmark (real media fixtures)
// ---------------------------------------------------------------------------

fn bench_video_decode(c: &mut Criterion) {
    tazama_media::init().ok();

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    if !fixtures.join("test_h264.mp4").exists() {
        eprintln!("SKIP video decode benchmarks: run scripts/generate-test-fixtures.sh first");
        return;
    }

    let mut group = c.benchmark_group("video_decode");
    // Decode fewer iterations since video is slower
    group.sample_size(20);

    // MP4 (H.264) via tarang
    let mp4_path = fixtures.join("test_h264.mp4");
    group.bench_function("tarang_mp4_10frames", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let config = tazama_media::decode::DecoderConfig {
                    path: black_box(mp4_path.clone()),
                };
                let decoder = tazama_media::decode::video::VideoDecoder::new(config);
                let range = tazama_media::decode::FrameRange { start: 0, end: 9 };
                let mut rx = decoder.decode(range).unwrap();
                let mut count = 0usize;
                while let Some(_frame) = rx.recv().await {
                    count += 1;
                }
                count
            })
        })
    });

    // MP4 via GStreamer
    let gst_mp4 = fixtures.join("test_h264_decode_gst.avi");
    std::fs::copy(&mp4_path, &gst_mp4).ok();
    group.bench_function("gstreamer_mp4_10frames", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let config = tazama_media::decode::DecoderConfig {
                    path: black_box(gst_mp4.clone()),
                };
                let decoder = tazama_media::decode::video::VideoDecoder::new(config);
                let range = tazama_media::decode::FrameRange { start: 0, end: 9 };
                let mut rx = decoder.decode(range).unwrap();
                let mut count = 0usize;
                while let Some(_frame) = rx.recv().await {
                    count += 1;
                }
                count
            })
        })
    });

    group.finish();
    let _ = std::fs::remove_file(&gst_mp4);
}

criterion_group!(
    benches,
    bench_probe,
    bench_audio_decode,
    bench_video_probe,
    bench_video_decode
);
criterion_main!(benches);
