use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyframeId(pub Uuid);

impl KeyframeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for KeyframeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyframeTrackId(pub Uuid);

impl KeyframeTrackId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for KeyframeTrackId {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpolation mode between keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Interpolation {
    Linear,
    Hold,
    BezierCubic {
        in_tangent: (f32, f32),
        out_tangent: (f32, f32),
    },
}

/// A single keyframe: a value at a specific frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub id: KeyframeId,
    pub frame: u64,
    pub value: f32,
    pub interpolation: Interpolation,
}

impl Keyframe {
    pub fn new(frame: u64, value: f32, interpolation: Interpolation) -> Self {
        Self {
            id: KeyframeId::new(),
            frame,
            value,
            interpolation,
        }
    }
}

/// A track of keyframes for a single parameter (e.g. "brightness", "volume").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeTrack {
    pub id: KeyframeTrackId,
    pub parameter: String,
    pub keyframes: Vec<Keyframe>,
}

impl KeyframeTrack {
    pub fn new(parameter: impl Into<String>) -> Self {
        Self {
            id: KeyframeTrackId::new(),
            parameter: parameter.into(),
            keyframes: Vec::new(),
        }
    }

    /// Add a keyframe, maintaining sort order by frame.
    pub fn add_keyframe(&mut self, kf: Keyframe) {
        self.keyframes.push(kf);
        self.keyframes.sort_by_key(|k| k.frame);
    }
}

/// Evaluate a keyframe track at a given frame, returning the interpolated value.
///
/// If `frame` is before the first keyframe, returns the first keyframe's value.
/// If `frame` is after the last keyframe, returns the last keyframe's value.
/// Between keyframes, interpolates according to the left keyframe's interpolation mode.
pub fn evaluate(track: &KeyframeTrack, frame: u64) -> Option<f32> {
    let kfs = &track.keyframes;
    if kfs.is_empty() {
        return None;
    }
    if kfs.len() == 1 || frame <= kfs[0].frame {
        return Some(kfs[0].value);
    }
    if frame >= kfs[kfs.len() - 1].frame {
        return Some(kfs[kfs.len() - 1].value);
    }

    // Binary search for the segment containing `frame`
    let idx = match kfs.binary_search_by_key(&frame, |k| k.frame) {
        Ok(i) => return Some(kfs[i].value),
        Err(i) => i - 1, // frame is between kfs[i-1] and kfs[i]
    };

    let left = &kfs[idx];
    let right = &kfs[idx + 1];
    let span = (right.frame - left.frame) as f32;
    if span == 0.0 {
        return Some(left.value);
    }
    let t = (frame - left.frame) as f32 / span;

    match left.interpolation {
        Interpolation::Hold => Some(left.value),
        Interpolation::Linear => Some(left.value + (right.value - left.value) * t),
        Interpolation::BezierCubic { out_tangent, .. } => {
            // Use the left keyframe's out_tangent and right keyframe's in_tangent
            let in_tangent = match right.interpolation {
                Interpolation::BezierCubic { in_tangent, .. } => in_tangent,
                _ => (0.0, 0.0),
            };
            Some(cubic_bezier_value(
                left.value,
                right.value,
                out_tangent,
                in_tangent,
                t,
            ))
        }
    }
}

/// Cubic bezier interpolation for keyframe values.
///
/// Control points:
/// P0 = left value, P3 = right value
/// P1 = P0 + out_tangent.1 (value offset)
/// P2 = P3 + in_tangent.1 (value offset)
fn cubic_bezier_value(
    v0: f32,
    v3: f32,
    out_tangent: (f32, f32),
    in_tangent: (f32, f32),
    t: f32,
) -> f32 {
    let v1 = v0 + out_tangent.1;
    let v2 = v3 + in_tangent.1;
    let mt = 1.0 - t;
    mt * mt * mt * v0 + 3.0 * mt * mt * t * v1 + 3.0 * mt * t * t * v2 + t * t * t * v3
}

/// Integrate the speed curve over a frame range to compute the source offset.
///
/// For speed ramping: instead of `local_frame * speed`, we integrate
/// the speed keyframe track from `start` to `end` to get the total
/// source frames consumed.
///
/// Uses trapezoidal integration for accuracy with non-linear speed curves.
pub fn integrated_speed(track: &KeyframeTrack, start: u64, end: u64) -> f64 {
    if start >= end {
        return 0.0;
    }
    // If no keyframes, assume speed = 1.0
    if track.keyframes.is_empty() {
        return (end - start) as f64;
    }

    let mut total = 0.0f64;
    for frame in start..end {
        let v0 = evaluate(track, frame).unwrap_or(1.0) as f64;
        let v1 = evaluate(track, frame + 1).unwrap_or(1.0) as f64;
        total += (v0 + v1) / 2.0;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear_track(keyframes: Vec<(u64, f32)>) -> KeyframeTrack {
        let mut track = KeyframeTrack::new("test");
        for (frame, value) in keyframes {
            track.add_keyframe(Keyframe::new(frame, value, Interpolation::Linear));
        }
        track
    }

    #[test]
    fn evaluate_empty_track() {
        let track = KeyframeTrack::new("test");
        assert_eq!(evaluate(&track, 0), None);
    }

    #[test]
    fn evaluate_single_keyframe() {
        let track = linear_track(vec![(10, 0.5)]);
        assert_eq!(evaluate(&track, 0), Some(0.5));
        assert_eq!(evaluate(&track, 10), Some(0.5));
        assert_eq!(evaluate(&track, 100), Some(0.5));
    }

    #[test]
    fn evaluate_linear_interpolation() {
        let track = linear_track(vec![(0, 0.0), (100, 1.0)]);
        assert_eq!(evaluate(&track, 0), Some(0.0));
        assert_eq!(evaluate(&track, 100), Some(1.0));
        let v = evaluate(&track, 50).unwrap();
        assert!((v - 0.5).abs() < 1e-6);
        let v = evaluate(&track, 25).unwrap();
        assert!((v - 0.25).abs() < 1e-6);
    }

    #[test]
    fn evaluate_hold_interpolation() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(0, 1.0, Interpolation::Hold));
        track.add_keyframe(Keyframe::new(100, 2.0, Interpolation::Hold));

        assert_eq!(evaluate(&track, 0), Some(1.0));
        assert_eq!(evaluate(&track, 50), Some(1.0));
        assert_eq!(evaluate(&track, 99), Some(1.0));
        assert_eq!(evaluate(&track, 100), Some(2.0));
    }

    #[test]
    fn evaluate_bezier_interpolation() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (0.33, 0.0),
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (-0.33, 0.0),
                out_tangent: (0.0, 0.0),
            },
        ));

        let v = evaluate(&track, 0).unwrap();
        assert!((v - 0.0).abs() < 1e-6);
        let v = evaluate(&track, 100).unwrap();
        assert!((v - 1.0).abs() < 1e-6);
        // Midpoint should be close to 0.5 with symmetric tangents
        let v = evaluate(&track, 50).unwrap();
        assert!(v > 0.0 && v < 1.0);
    }

    #[test]
    fn evaluate_before_first_keyframe() {
        let track = linear_track(vec![(10, 5.0), (20, 10.0)]);
        assert_eq!(evaluate(&track, 0), Some(5.0));
        assert_eq!(evaluate(&track, 5), Some(5.0));
    }

    #[test]
    fn evaluate_after_last_keyframe() {
        let track = linear_track(vec![(10, 5.0), (20, 10.0)]);
        assert_eq!(evaluate(&track, 30), Some(10.0));
        assert_eq!(evaluate(&track, 100), Some(10.0));
    }

    #[test]
    fn evaluate_exact_keyframe() {
        let track = linear_track(vec![(0, 0.0), (50, 5.0), (100, 10.0)]);
        assert_eq!(evaluate(&track, 50), Some(5.0));
    }

    #[test]
    fn evaluate_multiple_segments() {
        let track = linear_track(vec![(0, 0.0), (10, 10.0), (20, 0.0)]);
        let v = evaluate(&track, 5).unwrap();
        assert!((v - 5.0).abs() < 1e-6);
        let v = evaluate(&track, 15).unwrap();
        assert!((v - 5.0).abs() < 1e-6);
    }

    #[test]
    fn integrated_speed_constant() {
        let track = linear_track(vec![(0, 2.0), (100, 2.0)]);
        let result = integrated_speed(&track, 0, 10);
        assert!((result - 20.0).abs() < 1e-6);
    }

    #[test]
    fn integrated_speed_ramp() {
        let track = linear_track(vec![(0, 1.0), (10, 2.0)]);
        // Linear ramp from 1.0 to 2.0 over 10 frames: average speed = 1.5, total = 15.0
        let result = integrated_speed(&track, 0, 10);
        assert!((result - 15.0).abs() < 0.5);
    }

    #[test]
    fn integrated_speed_empty_track() {
        let track = KeyframeTrack::new("speed");
        let result = integrated_speed(&track, 0, 10);
        assert!((result - 10.0).abs() < 1e-6);
    }

    #[test]
    fn integrated_speed_zero_range() {
        let track = linear_track(vec![(0, 2.0)]);
        assert_eq!(integrated_speed(&track, 5, 5), 0.0);
        assert_eq!(integrated_speed(&track, 10, 5), 0.0);
    }

    #[test]
    fn keyframe_track_add_maintains_sort() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(50, 5.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(10, 1.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(30, 3.0, Interpolation::Linear));

        assert_eq!(track.keyframes[0].frame, 10);
        assert_eq!(track.keyframes[1].frame, 30);
        assert_eq!(track.keyframes[2].frame, 50);
    }

    #[test]
    fn keyframe_serde_round_trip() {
        let kf = Keyframe::new(10, 0.5, Interpolation::Linear);
        let json = serde_json::to_string(&kf).unwrap();
        let back: Keyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frame, 10);
        assert!((back.value - 0.5).abs() < 1e-6);
    }

    #[test]
    fn keyframe_track_serde_round_trip() {
        let mut track = KeyframeTrack::new("brightness");
        track.add_keyframe(Keyframe::new(0, 0.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(30, 1.0, Interpolation::Hold));
        let json = serde_json::to_string(&track).unwrap();
        let back: KeyframeTrack = serde_json::from_str(&json).unwrap();
        assert_eq!(back.parameter, "brightness");
        assert_eq!(back.keyframes.len(), 2);
    }

    #[test]
    fn two_keyframes_at_same_frame() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(10, 1.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(10, 2.0, Interpolation::Linear));
        // Both keyframes should be present (sort is stable by frame)
        assert_eq!(track.keyframes.len(), 2);
        assert_eq!(track.keyframes[0].frame, 10);
        assert_eq!(track.keyframes[1].frame, 10);
        // Evaluate at frame 10 should return one of them (binary_search finds exact match)
        let v = evaluate(&track, 10).unwrap();
        assert!(v == 1.0 || v == 2.0);
    }

    #[test]
    fn integrated_speed_single_keyframe() {
        // Single keyframe at frame 5 with speed 3.0
        let track = linear_track(vec![(5, 3.0)]);
        // Before, at, and after the keyframe should all evaluate to 3.0
        let result = integrated_speed(&track, 0, 10);
        // 10 frames * speed 3.0 = 30.0
        assert!((result - 30.0).abs() < 1e-6);
    }

    #[test]
    fn bezier_cubic_extreme_tangents() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (0.5, 100.0), // extreme overshoot
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (-0.5, -100.0), // extreme undershoot
                out_tangent: (0.0, 0.0),
            },
        ));

        // Endpoints should still be exact
        assert!((evaluate(&track, 0).unwrap() - 0.0).abs() < 1e-6);
        assert!((evaluate(&track, 100).unwrap() - 1.0).abs() < 1e-6);

        // Midpoint may overshoot/undershoot significantly with extreme tangents
        let mid = evaluate(&track, 50).unwrap();
        // Just verify it produces a finite value
        assert!(mid.is_finite());
    }

    #[test]
    fn evaluate_multi_segment_linear_three_segments() {
        // 4 keyframes = 3 segments
        let track = linear_track(vec![(0, 0.0), (10, 10.0), (20, 5.0), (30, 15.0)]);
        // Segment 1: 0→10, value 0→10
        let v = evaluate(&track, 5).unwrap();
        assert!((v - 5.0).abs() < 1e-6);
        // Segment 2: 10→20, value 10→5
        let v = evaluate(&track, 15).unwrap();
        assert!((v - 7.5).abs() < 1e-6);
        // Segment 3: 20→30, value 5→15
        let v = evaluate(&track, 25).unwrap();
        assert!((v - 10.0).abs() < 1e-6);
    }

    #[test]
    fn evaluate_just_before_and_after_keyframes() {
        let track = linear_track(vec![(10, 0.0), (20, 10.0), (30, 20.0)]);
        // Just before second keyframe
        let v = evaluate(&track, 19).unwrap();
        assert!((v - 9.0).abs() < 1e-6);
        // At second keyframe
        assert_eq!(evaluate(&track, 20), Some(10.0));
        // Just after second keyframe
        let v = evaluate(&track, 21).unwrap();
        assert!((v - 11.0).abs() < 1e-6);
    }

    #[test]
    fn integrated_speed_varying_ramp_1_to_3() {
        let track = linear_track(vec![(0, 1.0), (10, 3.0)]);
        // Linear ramp 1→3 over 10 frames, average = 2.0, total = 20.0
        let result = integrated_speed(&track, 0, 10);
        assert!((result - 20.0).abs() < 0.5);
    }

    #[test]
    fn integrated_speed_with_hold_interpolation() {
        let mut track = KeyframeTrack::new("speed");
        track.add_keyframe(Keyframe::new(0, 2.0, Interpolation::Hold));
        track.add_keyframe(Keyframe::new(10, 5.0, Interpolation::Hold));
        // With hold, speed stays at 2.0 from frame 0..9, then jumps to 5.0 at frame 10
        let result = integrated_speed(&track, 0, 10);
        // Each frame evaluates to 2.0 (hold), trapezoidal: (2+2)/2 * 9 + (2+5)/2 * 1 = 18 + 3.5 = 21.5
        // Actually frame 9→10: v0=eval(9)=2.0 (hold), v1=eval(10)=5.0, trap = (2+5)/2 = 3.5
        // frames 0..9: all 2.0, trap = 2.0 each = 9 * 2.0 = 18
        assert!((result - 21.5).abs() < 1e-6);
    }

    #[test]
    fn add_keyframe_duplicate_frames_maintains_both() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(5, 1.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(5, 2.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(5, 3.0, Interpolation::Linear));
        assert_eq!(track.keyframes.len(), 3);
        // All at same frame
        assert!(track.keyframes.iter().all(|k| k.frame == 5));
    }

    #[test]
    fn interpolation_serde_round_trip_all_variants() {
        let variants = vec![
            Interpolation::Linear,
            Interpolation::Hold,
            Interpolation::BezierCubic {
                in_tangent: (0.25, 0.1),
                out_tangent: (0.75, 0.9),
            },
        ];
        for interp in variants {
            let json = serde_json::to_string(&interp).unwrap();
            let back: Interpolation = serde_json::from_str(&json).unwrap();
            assert_eq!(back, interp);
        }
    }

    #[test]
    fn keyframe_id_uniqueness_and_default() {
        let id1 = KeyframeId::new();
        let id2 = KeyframeId::new();
        let id3 = KeyframeId::default();
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id2, id3);
    }

    #[test]
    fn bezier_cubic_very_large_tangents() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (0.5, 1e6),
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (-0.5, -1e6),
                out_tangent: (0.0, 0.0),
            },
        ));

        // Endpoints remain exact
        assert!((evaluate(&track, 0).unwrap() - 0.0).abs() < 1e-6);
        assert!((evaluate(&track, 100).unwrap() - 1.0).abs() < 1e-6);

        // Midpoint is finite despite extreme tangents
        let mid = evaluate(&track, 50).unwrap();
        assert!(mid.is_finite());
    }

    #[test]
    fn bezier_cubic_very_small_tangents() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (1e-9, 1e-9),
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (-1e-9, -1e-9),
                out_tangent: (0.0, 0.0),
            },
        ));

        let mid = evaluate(&track, 50).unwrap();
        // Near-zero tangents should behave close to linear
        assert!((mid - 0.5).abs() < 0.1, "near-zero tangent mid={mid}");
    }

    #[test]
    fn bezier_cubic_negative_tangents() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (-0.5, -2.0),
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (0.5, 2.0),
                out_tangent: (0.0, 0.0),
            },
        ));

        // Should produce finite values even with negative tangent directions
        for f in [10, 25, 50, 75, 90] {
            let v = evaluate(&track, f).unwrap();
            assert!(v.is_finite(), "negative tangent at frame {f} not finite");
        }
    }

    #[test]
    fn bezier_cubic_overshoot_exceeds_endpoint_values() {
        // Large tangents that cause clear value overshoot beyond [0, 1]
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (0.33, 50.0), // P1 = 50.0, far above endpoint
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (-0.33, -50.0), // P2 = -49.0, far below start
                out_tangent: (0.0, 0.0),
            },
        ));

        // The curve should overshoot: some samples should exceed 1.0 or go below 0.0
        let mut has_overshoot = false;
        for f in 1..100 {
            let v = evaluate(&track, f).unwrap();
            assert!(v.is_finite());
            if !(0.0..=1.0).contains(&v) {
                has_overshoot = true;
            }
        }
        assert!(
            has_overshoot,
            "expected bezier overshoot with extreme tangents"
        );
    }

    #[test]
    fn evaluate_same_frame_keyframes_returns_left_value() {
        // Two keyframes at the exact same frame — the div-by-zero guard should return left.value
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(50, 3.0, Interpolation::Linear));
        track.add_keyframe(Keyframe::new(50, 7.0, Interpolation::Linear));

        // binary_search may find either, but the span==0 guard returns left.value
        let v = evaluate(&track, 50).unwrap();
        assert!(v.is_finite());
        assert!(v == 3.0 || v == 7.0);
    }

    #[test]
    fn integrated_speed_varying_three_keyframes() {
        // Speed: 1.0 at frame 0, 3.0 at frame 5, 1.0 at frame 10
        let track = linear_track(vec![(0, 1.0), (5, 3.0), (10, 1.0)]);
        // Triangle speed curve: average speed = 2.0, over 10 frames = 20.0
        let result = integrated_speed(&track, 0, 10);
        assert!((result - 20.0).abs() < 0.5, "varying speed result={result}");
    }

    #[test]
    fn integrated_speed_partial_range() {
        // Speed ramp 1.0 -> 2.0 over frames 0..10, but only integrate frames 3..7
        let track = linear_track(vec![(0, 1.0), (10, 2.0)]);
        // At frame 3: speed=1.3, at frame 7: speed=1.7, average=1.5, 4 frames => 6.0
        let result = integrated_speed(&track, 3, 7);
        assert!((result - 6.0).abs() < 0.5, "partial range result={result}");
    }

    #[test]
    fn evaluate_at_exactly_first_keyframe() {
        let track = linear_track(vec![(10, 5.0), (20, 10.0), (30, 15.0)]);
        assert_eq!(evaluate(&track, 10), Some(5.0));
    }

    #[test]
    fn evaluate_at_exactly_last_keyframe() {
        let track = linear_track(vec![(10, 5.0), (20, 10.0), (30, 15.0)]);
        assert_eq!(evaluate(&track, 30), Some(15.0));
    }

    #[test]
    fn bezier_cubic_zero_tangents_behaves_like_linear() {
        let mut track = KeyframeTrack::new("test");
        track.add_keyframe(Keyframe::new(
            0,
            0.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (0.0, 0.0),
            },
        ));
        track.add_keyframe(Keyframe::new(
            100,
            1.0,
            Interpolation::BezierCubic {
                in_tangent: (0.0, 0.0),
                out_tangent: (0.0, 0.0),
            },
        ));

        // With zero tangents, P1=P0 and P2=P3, so it should behave similarly to linear
        let v = evaluate(&track, 50).unwrap();
        assert!((v - 0.5).abs() < 0.1, "zero tangent bezier mid={v}");
    }
}
