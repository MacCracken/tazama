use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_eq(c: &mut Criterion) {
    let samples: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.01).sin()).collect();
    c.bench_function("eq_3band_1s_48khz", |b| {
        b.iter(|| {
            let mut buf = samples.clone();
            tazama_media::dsp::eq::apply_eq(
                black_box(&mut buf),
                48000,
                2,
                black_box(3.0),
                black_box(-2.0),
                black_box(1.5),
            );
        })
    });
}

fn bench_compressor(c: &mut Criterion) {
    let samples: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.01).sin()).collect();
    c.bench_function("compressor_1s_48khz", |b| {
        b.iter(|| {
            let mut buf = samples.clone();
            tazama_media::dsp::compressor::apply_compressor(
                black_box(&mut buf),
                48000,
                2,
                black_box(-20.0),
                black_box(4.0),
                black_box(5.0),
                black_box(50.0),
            );
        })
    });
}

fn bench_noise_reduction(c: &mut Criterion) {
    let samples: Vec<f32> = (0..48000)
        .map(|i| (i as f32 * 0.01).sin() + (i as f32 * 0.1).sin() * 0.1)
        .collect();
    c.bench_function("noise_reduction_1s_48khz", |b| {
        b.iter(|| {
            let mut buf = samples.clone();
            tazama_media::dsp::noise_reduction::apply_noise_reduction(
                black_box(&mut buf),
                2,
                black_box(0.5),
            );
        })
    });
}

fn bench_reverb(c: &mut Criterion) {
    let samples: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.01).sin()).collect();
    c.bench_function("reverb_1s_48khz", |b| {
        b.iter(|| {
            let mut buf = samples.clone();
            tazama_media::dsp::reverb::apply_reverb(
                black_box(&mut buf),
                48000,
                2,
                black_box(0.7),
                black_box(0.5),
                black_box(0.3),
            );
        })
    });
}

criterion_group!(
    benches,
    bench_eq,
    bench_compressor,
    bench_noise_reduction,
    bench_reverb
);
criterion_main!(benches);
