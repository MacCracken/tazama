use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tazama_core::clip::{Clip, ClipKind};
use tazama_core::keyframe::{Interpolation, Keyframe, KeyframeTrack, evaluate, integrated_speed};
use tazama_core::timeline::{Timeline, Track, TrackKind};

// ---------------------------------------------------------------------------
// Keyframe evaluation benchmarks
// ---------------------------------------------------------------------------

fn make_linear_track(n: usize) -> KeyframeTrack {
    let mut track = KeyframeTrack::new("bench");
    for i in 0..n {
        track.add_keyframe(Keyframe::new(
            i as u64 * 10,
            (i as f32 * 0.1).sin(),
            Interpolation::Linear,
        ));
    }
    track
}

fn bench_evaluate(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyframe_evaluate");

    for &count in &[10, 100, 1000] {
        let track = make_linear_track(count);
        let mid_frame = (count as u64 * 10) / 2 + 3; // hit an interpolation point
        group.bench_function(format!("{count}_keyframes"), |b| {
            b.iter(|| evaluate(black_box(&track), black_box(mid_frame)))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Integrated speed benchmarks
// ---------------------------------------------------------------------------

fn bench_integrated_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrated_speed");

    let track = make_linear_track(20);

    for &length in &[100, 500, 2000] {
        group.bench_function(format!("len_{length}"), |b| {
            b.iter(|| integrated_speed(black_box(&track), 0, black_box(length)))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Timeline serde benchmarks
// ---------------------------------------------------------------------------

fn build_timeline(num_tracks: usize, clips_per_track: usize) -> Timeline {
    let mut timeline = Timeline::new();
    for t in 0..num_tracks {
        let kind = if t % 2 == 0 {
            TrackKind::Video
        } else {
            TrackKind::Audio
        };
        let clip_kind = if t % 2 == 0 {
            ClipKind::Video
        } else {
            ClipKind::Audio
        };
        let mut track = Track::new(format!("Track {t}"), kind);
        for i in 0..clips_per_track {
            let start = (i as u64) * 100;
            let clip = Clip::new(format!("Clip {t}-{i}"), clip_kind, start, 90);
            track.add_clip(clip).unwrap();
        }
        timeline.add_track(track);
    }
    timeline
}

fn bench_timeline_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("timeline_serde");

    let timeline = build_timeline(10, 100);
    let json = serde_json::to_string(&timeline).unwrap();

    group.bench_function("serialize_10t_100c", |b| {
        b.iter(|| serde_json::to_string(black_box(&timeline)).unwrap())
    });

    group.bench_function("deserialize_10t_100c", |b| {
        b.iter(|| serde_json::from_str::<Timeline>(black_box(&json)).unwrap())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_evaluate,
    bench_integrated_speed,
    bench_timeline_serde
);
criterion_main!(benches);
