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

criterion_group!(benches, bench_probe, bench_audio_decode);
criterion_main!(benches);
