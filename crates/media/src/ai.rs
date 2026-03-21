//! AI-powered editing features built on tarang-ai.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tarang::ai::scene::compute_luminance_histogram;
use tarang::ai::{
    SceneBoundary, SceneBoundaryType, SceneDetectionConfig, SceneDetector, content_score,
};
use tarang::core::VideoFrame;

use crate::error::MediaPipelineError;
use crate::thumbnail::{create_demuxer, find_video_stream};

// ---------------------------------------------------------------------------
// Auto-cut / Highlights
// ---------------------------------------------------------------------------

/// A scored segment of video suitable for a highlight reel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub start_ms: u64,
    pub end_ms: u64,
    pub score: f64,
}

/// Detect highlight segments in a video file.
///
/// Decodes the video, runs scene detection, scores frames for visual interest,
/// and returns the top `max_highlights` segments ranked by score.
pub async fn detect_highlights(
    path: &Path,
    max_highlights: usize,
) -> Result<Vec<Highlight>, MediaPipelineError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || detect_highlights_sync(&path, max_highlights))
        .await
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?
}

fn detect_highlights_sync(
    path: &Path,
    max_highlights: usize,
) -> Result<Vec<Highlight>, MediaPipelineError> {
    let mut demuxer = create_demuxer(path)?;
    let info = demuxer.probe()?;

    let (video_stream_idx, codec) = find_video_stream(&info)
        .ok_or_else(|| MediaPipelineError::Decode("no video stream".into()))?;

    let config = tarang::video::DecoderConfig::for_codec(codec)?;
    let mut decoder = tarang::video::VideoDecoder::new(config)?;
    if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
        decoder.init(vs);
    }

    let mut scene_detector = SceneDetector::new(SceneDetectionConfig::default());
    let mut boundaries: Vec<SceneBoundary> = Vec::new();
    // Score accumulator per scene: (start_ms, sum_score, frame_count)
    let mut scene_scores: Vec<(u64, f64, u32)> = Vec::new();
    let mut current_scene_start_ms = 0u64;
    let mut current_score_sum = 0.0f64;
    let mut current_frame_count = 0u32;

    // Sample every 5th frame for performance
    let mut frame_idx = 0u64;

    loop {
        let packet = match demuxer.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.stream_index != video_stream_idx {
            continue;
        }
        decoder.send_packet(&packet.data, packet.timestamp)?;

        while let Ok(frame) = decoder.receive_frame() {
            let ts_ms = frame.timestamp.as_millis() as u64;

            if let Some(boundary) = scene_detector.feed_frame(&frame) {
                // Close previous scene
                if current_frame_count > 0 {
                    scene_scores.push((
                        current_scene_start_ms,
                        current_score_sum / current_frame_count as f64,
                        current_frame_count,
                    ));
                }
                boundaries.push(boundary);
                current_scene_start_ms = ts_ms;
                current_score_sum = 0.0;
                current_frame_count = 0;
            }

            // Score every 5th frame
            if frame_idx % 5 == 0 {
                current_score_sum += content_score(&frame) as f64;
                current_frame_count += 1;
            }
            frame_idx += 1;
        }
    }

    // Flush
    let _ = decoder.flush();
    while let Ok(frame) = decoder.receive_frame() {
        let ts_ms = frame.timestamp.as_millis() as u64;
        if let Some(boundary) = scene_detector.feed_frame(&frame) {
            if current_frame_count > 0 {
                scene_scores.push((
                    current_scene_start_ms,
                    current_score_sum / current_frame_count as f64,
                    current_frame_count,
                ));
            }
            boundaries.push(boundary);
            current_scene_start_ms = ts_ms;
            current_score_sum = 0.0;
            current_frame_count = 0;
        }
        if frame_idx % 5 == 0 {
            current_score_sum += content_score(&frame) as f64;
            current_frame_count += 1;
        }
        frame_idx += 1;
    }

    // Close last scene
    let final_boundaries = scene_detector.finish();
    boundaries.extend(final_boundaries);
    if current_frame_count > 0 {
        scene_scores.push((
            current_scene_start_ms,
            current_score_sum / current_frame_count as f64,
            current_frame_count,
        ));
    }

    // Build highlight segments from scene scores
    // Each scene becomes a highlight candidate; end time = next scene start or last boundary
    let mut highlights: Vec<Highlight> = Vec::new();
    for (i, &(start_ms, avg_score, _count)) in scene_scores.iter().enumerate() {
        let end_ms = if i + 1 < scene_scores.len() {
            scene_scores[i + 1].0
        } else {
            // Estimate from last boundary or add 5 seconds
            boundaries
                .last()
                .map(|b| b.timestamp.as_millis() as u64)
                .unwrap_or(start_ms + 5000)
        };
        if end_ms > start_ms {
            highlights.push(Highlight {
                start_ms,
                end_ms,
                score: avg_score,
            });
        }
    }

    // Sort by score descending, take top N
    highlights.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    highlights.truncate(max_highlights);
    // Re-sort by time for output
    highlights.sort_by_key(|h| h.start_ms);

    Ok(highlights)
}

// ---------------------------------------------------------------------------
// Subtitle Generation (SRT/VTT from transcription)
// ---------------------------------------------------------------------------

/// A subtitle cue with timing and text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleCue {
    pub index: usize,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Generate SRT-formatted subtitles from transcription segments.
pub fn segments_to_srt(segments: &[SubtitleCue]) -> String {
    let mut out = String::new();
    for cue in segments {
        out.push_str(&format!("{}\n", cue.index));
        out.push_str(&format!(
            "{} --> {}\n",
            format_srt_time(cue.start_ms),
            format_srt_time(cue.end_ms),
        ));
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }
    out
}

/// Generate WebVTT-formatted subtitles from transcription segments.
pub fn segments_to_vtt(segments: &[SubtitleCue]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for cue in segments {
        out.push_str(&format!(
            "{} --> {}\n",
            format_vtt_time(cue.start_ms),
            format_vtt_time(cue.end_ms),
        ));
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }
    out
}

fn format_srt_time(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1_000;
    let ms = ms % 1_000;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

fn format_vtt_time(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1_000;
    let ms = ms % 1_000;
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

// ---------------------------------------------------------------------------
// AI Color Correction
// ---------------------------------------------------------------------------

/// Per-channel color correction gains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorCorrection {
    pub brightness_offset: f32,
    pub contrast_factor: f32,
    pub saturation_factor: f32,
}

/// Analyze a video frame's luminance histogram and suggest color correction.
///
/// Returns correction values that can be applied as a ColorGrade effect.
pub fn auto_color_correct(frame: &VideoFrame) -> ColorCorrection {
    let histogram = compute_luminance_histogram(frame, 256);

    // Compute mean luminance
    let total: f64 = histogram.iter().sum();
    let mean: f64 = if total > 0.0 {
        histogram
            .iter()
            .enumerate()
            .map(|(i, &v)| i as f64 * v)
            .sum::<f64>()
            / total
    } else {
        128.0
    };

    // Compute standard deviation
    let variance: f64 = if total > 0.0 {
        histogram
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let diff = i as f64 - mean;
                diff * diff * v
            })
            .sum::<f64>()
            / total
    } else {
        0.0
    };
    let std_dev = variance.sqrt();

    // Target: mean ~128 (neutral), std_dev ~50 (good contrast)
    let target_mean = 128.0;
    let target_std_dev = 50.0;

    // Brightness: shift mean toward target
    let brightness_offset = ((target_mean - mean) / 255.0) as f32;

    // Contrast: scale to target std_dev
    let contrast_factor = if std_dev > 1.0 {
        ((target_std_dev / std_dev) as f32).clamp(0.5, 2.0)
    } else {
        1.0
    };

    // Saturation: boost if image is very flat, reduce if over-saturated
    // (approximation based on luminance spread)
    let saturation_factor = if std_dev < 30.0 {
        1.2 // boost flat images
    } else if std_dev > 70.0 {
        0.9 // tame over-contrasty images
    } else {
        1.0
    };

    ColorCorrection {
        brightness_offset,
        contrast_factor,
        saturation_factor,
    }
}

// ---------------------------------------------------------------------------
// Smart Transition Suggestions
// ---------------------------------------------------------------------------

/// A suggested transition between two clips.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSuggestion {
    pub kind: String,
    pub duration_frames: u64,
    pub reason: String,
}

/// Suggest a transition type based on scene boundary characteristics.
///
/// Uses the change score and boundary type to recommend an appropriate transition.
pub fn suggest_transition(boundary: &SceneBoundary, fps: f64) -> TransitionSuggestion {
    match boundary.boundary_type {
        SceneBoundaryType::HardCut => {
            if boundary.change_score > 0.8 {
                // Very dramatic change → quick cut or wipe
                TransitionSuggestion {
                    kind: "Cut".to_string(),
                    duration_frames: 0,
                    reason: "High contrast scene change — clean cut works best".to_string(),
                }
            } else {
                // Moderate hard cut → short dissolve
                let dur = (fps * 0.5).round() as u64; // 0.5 second
                TransitionSuggestion {
                    kind: "Dissolve".to_string(),
                    duration_frames: dur.max(1),
                    reason: "Moderate scene change — short dissolve smooths the edit".to_string(),
                }
            }
        }
        SceneBoundaryType::GradualTransition => {
            if boundary.change_score > 0.5 {
                let dur = (fps * 1.0).round() as u64;
                TransitionSuggestion {
                    kind: "Dissolve".to_string(),
                    duration_frames: dur.max(1),
                    reason: "Gradual scene shift — dissolve matches the natural pace".to_string(),
                }
            } else {
                let dur = (fps * 1.5).round() as u64;
                TransitionSuggestion {
                    kind: "Fade".to_string(),
                    duration_frames: dur.max(1),
                    reason: "Subtle scene change — fade creates a gentle mood transition"
                        .to_string(),
                }
            }
        }
        _ => {
            let dur = (fps * 0.5).round() as u64;
            TransitionSuggestion {
                kind: "Dissolve".to_string(),
                duration_frames: dur.max(1),
                reason: "Scene boundary detected — dissolve as default".to_string(),
            }
        }
    }
}

/// Analyze a video file and suggest transitions at each scene boundary.
pub async fn suggest_transitions(
    path: &Path,
    fps: f64,
) -> Result<Vec<(u64, TransitionSuggestion)>, MediaPipelineError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut demuxer = create_demuxer(&path)?;
        let info = demuxer.probe()?;
        let (video_stream_idx, codec) = find_video_stream(&info)
            .ok_or_else(|| MediaPipelineError::Decode("no video stream".into()))?;

        let config = tarang::video::DecoderConfig::for_codec(codec)?;
        let mut decoder = tarang::video::VideoDecoder::new(config)?;
        if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
            decoder.init(vs);
        }

        let mut scene_detector = SceneDetector::new(SceneDetectionConfig::default());

        loop {
            let packet = match demuxer.next_packet() {
                Ok(p) => p,
                Err(_) => break,
            };
            if packet.stream_index != video_stream_idx {
                continue;
            }
            decoder.send_packet(&packet.data, packet.timestamp)?;
            while let Ok(frame) = decoder.receive_frame() {
                scene_detector.feed_frame(&frame);
            }
        }

        let _ = decoder.flush();
        while let Ok(frame) = decoder.receive_frame() {
            scene_detector.feed_frame(&frame);
        }

        let boundaries = scene_detector.finish();
        let suggestions: Vec<(u64, TransitionSuggestion)> = boundaries
            .iter()
            .map(|b| {
                let ts_ms = b.timestamp.as_millis() as u64;
                (ts_ms, suggest_transition(b, fps))
            })
            .collect();

        Ok(suggestions)
    })
    .await
    .map_err(|e| MediaPipelineError::Decode(e.to_string()))?
}

// ---------------------------------------------------------------------------
// LLM-powered features (via hoosh)
// ---------------------------------------------------------------------------

/// Configuration for hoosh LLM calls.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("HOOSH_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8088".to_string()),
            model: std::env::var("HOOSH_MODEL").unwrap_or_else(|_| "llama3".to_string()),
            api_key: std::env::var("HOOSH_API_KEY").ok(),
        }
    }
}

/// A clip description generated by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipDescription {
    pub summary: String,
    pub tags: Vec<String>,
}

/// Refined subtitle cue with cleaned-up text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinedSubtitles {
    pub cues: Vec<SubtitleCue>,
    pub language: String,
}

async fn llm_chat(
    config: &LlmConfig,
    system: &str,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, MediaPipelineError> {
    let url = format!("{}/v1/chat/completions", config.endpoint);

    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": prompt },
        ],
        "temperature": 0.3,
        "max_tokens": max_tokens,
    });

    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&body);
    if let Some(key) = &config.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| MediaPipelineError::Export(format!("hoosh request failed: {e}")))?;

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| MediaPipelineError::Export(format!("hoosh response parse: {e}")))?;

    result["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| MediaPipelineError::Export("empty LLM response".into()))
}

/// Generate a clip description using LLM analysis of transcription and metadata.
pub async fn describe_clip(
    config: &LlmConfig,
    transcription: &[SubtitleCue],
    duration_ms: u64,
    has_video: bool,
) -> Result<ClipDescription, MediaPipelineError> {
    let transcript_text: String = transcription
        .iter()
        .map(|c| format!("[{}] {}", format_srt_time(c.start_ms), c.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "A {} clip, {} seconds long.\n\nTranscription:\n{}\n\n\
         Generate a JSON object with:\n\
         - \"summary\": one sentence description (max 100 chars)\n\
         - \"tags\": array of 3-5 relevant tags\n\
         Respond with ONLY the JSON object, no other text.",
        if has_video { "video" } else { "audio" },
        duration_ms / 1000,
        if transcript_text.len() > 2000 {
            &transcript_text[..2000]
        } else {
            &transcript_text
        },
    );

    let response = llm_chat(
        config,
        "You are a media content analyzer. Output only valid JSON.",
        &prompt,
        256,
    )
    .await?;

    // Parse JSON from response, with fallback
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
        let summary = parsed["summary"]
            .as_str()
            .unwrap_or("No description available")
            .to_string();
        let tags = parsed["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(ClipDescription { summary, tags })
    } else {
        // LLM didn't return valid JSON, use response as summary
        Ok(ClipDescription {
            summary: response.chars().take(100).collect(),
            tags: vec![],
        })
    }
}

/// Refine raw Whisper transcription using LLM (fix grammar, remove fillers).
pub async fn refine_subtitles(
    config: &LlmConfig,
    cues: &[SubtitleCue],
) -> Result<Vec<SubtitleCue>, MediaPipelineError> {
    if cues.is_empty() {
        return Ok(vec![]);
    }

    let cue_text: String = cues
        .iter()
        .map(|c| format!("{}: {}", c.index, c.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Here are auto-generated subtitle cues from speech recognition.\n\
         Fix grammar, remove filler words (um, uh, like), and improve readability.\n\
         Keep the same number of cues and their numbering.\n\
         Return ONLY the refined text, one line per cue in the format \"N: text\".\n\n\
         {cue_text}"
    );

    let response = llm_chat(
        config,
        "You are a subtitle editor. Clean up speech-to-text output for readability. Preserve meaning and timing boundaries.",
        &prompt,
        1024,
    )
    .await?;

    // Parse refined lines back into cues, keeping original timing
    let mut refined = cues.to_vec();
    for line in response.lines() {
        let line = line.trim();
        if let Some((idx_str, text)) = line.split_once(':') {
            if let Ok(idx) = idx_str.trim().parse::<usize>() {
                if let Some(cue) = refined.iter_mut().find(|c| c.index == idx) {
                    cue.text = text.trim().to_string();
                }
            }
        }
    }

    Ok(refined)
}

/// Translate subtitles to a target language using LLM.
pub async fn translate_subtitles(
    config: &LlmConfig,
    cues: &[SubtitleCue],
    target_language: &str,
) -> Result<Vec<SubtitleCue>, MediaPipelineError> {
    if cues.is_empty() {
        return Ok(vec![]);
    }

    let cue_text: String = cues
        .iter()
        .map(|c| format!("{}: {}", c.index, c.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Translate these subtitle cues to {target_language}.\n\
         Keep the same numbering. Return ONLY translated text, one line per cue in the format \"N: text\".\n\n\
         {cue_text}"
    );

    let response = llm_chat(
        config,
        &format!("You are a professional subtitle translator. Translate accurately to {target_language}, preserving tone and timing boundaries."),
        &prompt,
        1024,
    )
    .await?;

    let mut translated = cues.to_vec();
    for line in response.lines() {
        let line = line.trim();
        if let Some((idx_str, text)) = line.split_once(':') {
            if let Ok(idx) = idx_str.trim().parse::<usize>() {
                if let Some(cue) = translated.iter_mut().find(|c| c.index == idx) {
                    cue.text = text.trim().to_string();
                }
            }
        }
    }

    Ok(translated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn srt_format_basic() {
        let cues = vec![
            SubtitleCue {
                index: 1,
                start_ms: 0,
                end_ms: 2500,
                text: "Hello world".into(),
            },
            SubtitleCue {
                index: 2,
                start_ms: 3000,
                end_ms: 5500,
                text: "Second line".into(),
            },
        ];
        let srt = segments_to_srt(&cues);
        assert!(srt.contains("00:00:00,000 --> 00:00:02,500"));
        assert!(srt.contains("Hello world"));
        assert!(srt.contains("00:00:03,000 --> 00:00:05,500"));
    }

    #[test]
    fn vtt_format_basic() {
        let cues = vec![SubtitleCue {
            index: 1,
            start_ms: 1000,
            end_ms: 3000,
            text: "Test".into(),
        }];
        let vtt = segments_to_vtt(&cues);
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("00:00:01.000 --> 00:00:03.000"));
    }

    #[test]
    fn srt_time_formatting() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
        assert_eq!(format_srt_time(3661500), "01:01:01,500");
        assert_eq!(format_srt_time(999), "00:00:00,999");
    }

    #[test]
    fn transition_suggestion_hard_cut_high() {
        let boundary = SceneBoundary {
            timestamp: Duration::from_secs(5),
            frame_index: 150,
            change_score: 0.9,
            boundary_type: SceneBoundaryType::HardCut,
        };
        let suggestion = suggest_transition(&boundary, 30.0);
        assert_eq!(suggestion.kind, "Cut");
        assert_eq!(suggestion.duration_frames, 0);
    }

    #[test]
    fn transition_suggestion_gradual_low() {
        let boundary = SceneBoundary {
            timestamp: Duration::from_secs(10),
            frame_index: 300,
            change_score: 0.3,
            boundary_type: SceneBoundaryType::GradualTransition,
        };
        let suggestion = suggest_transition(&boundary, 24.0);
        assert_eq!(suggestion.kind, "Fade");
        assert!(suggestion.duration_frames > 0);
    }
}
