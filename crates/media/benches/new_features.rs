//! Benchmarks for new tarang 0.20.3 integration features.
//!
//! Run with:
//!   cargo bench -p tazama-media --bench new_features

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;

fn make_wav_bytes(sample_rate: u32, duration_secs: u32) -> Vec<u8> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let num_samples = sample_rate * duration_secs;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = num_samples * num_channels as u32 * bits_per_sample as u32 / 8;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + data_size as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
        let pcm = (sample * i16::MAX as f64) as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }
    buf
}

// ---------------------------------------------------------------------------
// Pixel conversion: tarang vs manual
// ---------------------------------------------------------------------------

fn bench_pixel_conversion(c: &mut Criterion) {
    use bytes::Bytes;

    let w = 1920u32;
    let h = 1080u32;
    // Generate RGBA test frame
    let rgba: Vec<u8> = (0..(w * h * 4) as usize)
        .map(|i| (i % 256) as u8)
        .collect();

    let mut group = c.benchmark_group("pixel_conversion");

    // RGBA → YUV420p via tarang (rgb24_to_yuv420p)
    group.bench_function("tarang_rgba_to_yuv_1080p", |b| {
        b.iter(|| {
            let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|c| &c[..3]).copied().collect();
            let rgb_frame = tarang::core::VideoFrame {
                data: Bytes::from(rgb),
                pixel_format: tarang::core::PixelFormat::Rgb24,
                width: w,
                height: h,
                timestamp: std::time::Duration::ZERO,
            };
            let result = tarang::video::convert::rgb24_to_yuv420p(black_box(&rgb_frame)).unwrap();
            black_box(result.data.len());
        })
    });

    // YUV420p → RGBA via tarang (yuv420p_to_rgb24)
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|c| &c[..3]).copied().collect();
    let rgb_frame = tarang::core::VideoFrame {
        data: Bytes::from(rgb),
        pixel_format: tarang::core::PixelFormat::Rgb24,
        width: w,
        height: h,
        timestamp: std::time::Duration::ZERO,
    };
    let yuv_frame = tarang::video::convert::rgb24_to_yuv420p(&rgb_frame).unwrap();

    group.bench_function("tarang_yuv_to_rgb_1080p", |b| {
        b.iter(|| {
            let result = tarang::video::convert::yuv420p_to_rgb24(black_box(&yuv_frame)).unwrap();
            black_box(result.data.len());
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Loudness measurement
// ---------------------------------------------------------------------------

fn bench_loudness(c: &mut Criterion) {
    // 1 second of 48kHz stereo audio
    let samples: Vec<f32> = (0..96000)
        .map(|i| (i as f32 * 0.01).sin() * 0.5)
        .collect();
    let buf = tazama_media::AudioBuffer {
        sample_rate: 48000,
        channels: 2,
        samples: samples.clone(),
        timestamp_ns: 0,
    };

    let mut group = c.benchmark_group("loudness");

    group.bench_function("measure_1s_stereo", |b| {
        b.iter(|| {
            black_box(tazama_media::loudness::measure_loudness(black_box(&buf)));
        })
    });

    group.bench_function("normalize_1s_stereo", |b| {
        b.iter(|| {
            let mut s = samples.clone();
            tazama_media::loudness::normalize_audio(
                black_box(&mut s),
                2,
                48000,
                -14.0,
            );
            black_box(s.len());
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Waveform extraction
// ---------------------------------------------------------------------------

fn bench_waveform(c: &mut Criterion) {
    tazama_media::init().ok();

    let dir = std::env::temp_dir().join("tazama_bench_waveform");
    std::fs::create_dir_all(&dir).unwrap();

    let wav_path = dir.join("bench_waveform.wav");
    let data = make_wav_bytes(48000, 5); // 5 seconds
    std::fs::write(&wav_path, data).unwrap();

    let mut group = c.benchmark_group("waveform");
    group.sample_size(20);

    group.bench_function("extract_5s_100pps", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(
                tazama_media::waveform::extract_waveform(black_box(&wav_path), 100),
            )
            .unwrap()
        })
    });

    group.bench_function("extract_5s_200pps", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(
                tazama_media::waveform::extract_waveform(black_box(&wav_path), 200),
            )
            .unwrap()
        })
    });

    group.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// AI: color correction
// ---------------------------------------------------------------------------

fn bench_ai_color(c: &mut Criterion) {
    use bytes::Bytes;

    let w = 1920u32;
    let h = 1080u32;

    // Create a YUV420p test frame
    let y_size = (w * h) as usize;
    let uv_size = ((w / 2) * (h / 2)) as usize;
    let mut yuv = vec![128u8; y_size + 2 * uv_size];
    // Add some variation
    for i in 0..y_size {
        yuv[i] = ((i * 7) % 256) as u8;
    }

    let frame = tarang::core::VideoFrame {
        data: Bytes::from(yuv),
        pixel_format: tarang::core::PixelFormat::Yuv420p,
        width: w,
        height: h,
        timestamp: std::time::Duration::ZERO,
    };

    let mut group = c.benchmark_group("ai");

    group.bench_function("auto_color_1080p", |b| {
        b.iter(|| {
            black_box(tazama_media::ai::auto_color_correct(black_box(&frame)));
        })
    });

    // Content scoring
    group.bench_function("content_score_1080p", |b| {
        b.iter(|| {
            black_box(tarang::ai::content_score(black_box(&frame)));
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// AI: highlight detection on real video
// ---------------------------------------------------------------------------

fn bench_ai_highlights(c: &mut Criterion) {
    tazama_media::init().ok();

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mp4_path = fixtures.join("test_h264.mp4");

    if !mp4_path.exists() {
        eprintln!("SKIP highlight benchmarks: run scripts/generate-test-fixtures.sh first");
        return;
    }

    let mut group = c.benchmark_group("ai_highlights");
    group.sample_size(10);

    // Quick check that the fixture has a video stream
    let rt_check = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let can_run = rt_check
        .block_on(tazama_media::ai::detect_highlights(&mp4_path, 1))
        .is_ok();
    drop(rt_check);

    if !can_run {
        eprintln!("SKIP highlight benchmarks: fixture has no decodable video stream");
        group.finish();
        return;
    }

    group.bench_function("detect_highlights_mp4", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::ai::detect_highlights(
                black_box(&mp4_path),
                5,
            ))
            .unwrap()
        })
    });

    group.bench_function("suggest_transitions_mp4", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(tazama_media::ai::suggest_transitions(
                black_box(&mp4_path),
                30.0,
            ))
            .unwrap()
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Video scaling
// ---------------------------------------------------------------------------

fn bench_video_scale(c: &mut Criterion) {
    use bytes::Bytes;
    use tarang::video::scale::{scale_frame, ScaleFilter};

    let w = 1920u32;
    let h = 1080u32;
    let rgb = vec![128u8; (w * h * 3) as usize];

    let frame = tarang::core::VideoFrame {
        data: Bytes::from(rgb),
        pixel_format: tarang::core::PixelFormat::Rgb24,
        width: w,
        height: h,
        timestamp: std::time::Duration::ZERO,
    };

    let mut group = c.benchmark_group("video_scale");

    group.bench_function("1080p_to_720p_bilinear", |b| {
        b.iter(|| {
            let result = scale_frame(black_box(&frame), 1280, 720, ScaleFilter::Bilinear).unwrap();
            black_box(result.data.len());
        })
    });

    group.bench_function("1080p_to_720p_lanczos3", |b| {
        b.iter(|| {
            let result = scale_frame(black_box(&frame), 1280, 720, ScaleFilter::Lanczos3).unwrap();
            black_box(result.data.len());
        })
    });

    group.bench_function("1080p_to_thumbnail_128x72", |b| {
        b.iter(|| {
            let result = scale_frame(black_box(&frame), 128, 72, ScaleFilter::Bilinear).unwrap();
            black_box(result.data.len());
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pixel_conversion,
    bench_loudness,
    bench_waveform,
    bench_ai_color,
    bench_ai_highlights,
    bench_video_scale,
);
criterion_main!(benches);
