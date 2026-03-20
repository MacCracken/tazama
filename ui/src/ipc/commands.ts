import { invoke } from "@tauri-apps/api/core";
import type { Project, MediaInfo, ExportConfig, HardwareInfo, ExportEncoder, ThumbnailSpec, WaveformData } from "../types";

export async function newProject(
  name: string,
  width: number,
  height: number,
): Promise<Project> {
  return invoke<Project>("new_project", { name, width, height });
}

export async function openProject(path: string): Promise<Project> {
  return invoke<Project>("open_project", { path });
}

export async function saveProject(
  project: Project,
  path: string,
): Promise<void> {
  return invoke<void>("save_project", { project, path });
}

export async function importMedia(
  projectRoot: string,
  source: string,
): Promise<string> {
  return invoke<string>("import_media", {
    projectRoot,
    source,
  });
}

export async function probeMedia(path: string): Promise<MediaInfo> {
  return invoke<MediaInfo>("probe_media", { path });
}

export async function exportProject(
  project: Project,
  config: ExportConfig,
): Promise<void> {
  return invoke<void>("export_project", { project, config });
}

export interface PreviewFrame {
  data: string; // base64-encoded RGBA
  width: number;
  height: number;
}

export async function renderPreviewFrame(
  project: Project,
  frameIndex: number,
): Promise<PreviewFrame> {
  return invoke<PreviewFrame>("render_preview_frame", {
    project,
    frameIndex,
  });
}

// Autosave commands
export async function startAutosave(): Promise<void> {
  return invoke<void>("start_autosave");
}

export async function stopAutosave(): Promise<void> {
  return invoke<void>("stop_autosave");
}

export async function checkAutosaveRecovery(
  path: string,
): Promise<Project | null> {
  return invoke<Project | null>("check_autosave_recovery", { path });
}

export async function cleanupAutosave(path: string): Promise<void> {
  return invoke<void>("cleanup_autosave", { path });
}

export async function notifyAutosave(
  project: Project,
  path: string,
): Promise<void> {
  return invoke<void>("notify_autosave", { project, path });
}

// Recording commands
export async function startRecording(
  sampleRate: number,
  channels: number,
): Promise<void> {
  return invoke<void>("start_recording", { sampleRate, channels });
}

export async function stopRecording(): Promise<string> {
  return invoke<string>("stop_recording");
}

// Proxy commands
export async function generateProxies(
  project: Project,
  proxyDir: string,
  targetWidth: number,
): Promise<string[]> {
  return invoke<string[]>("generate_proxies", {
    project,
    proxyDir,
    targetWidth,
  });
}

export async function setProxyMode(enabled: boolean): Promise<void> {
  return invoke<void>("set_proxy_mode", { enabled });
}

// AI features

export interface Highlight {
  start_ms: number;
  end_ms: number;
  score: number;
}

export interface SubtitleCue {
  index: number;
  start_ms: number;
  end_ms: number;
  text: string;
}

export interface ColorCorrection {
  brightness_offset: number;
  contrast_factor: number;
  saturation_factor: number;
}

export interface TransitionSuggestion {
  kind: string;
  duration_frames: number;
  reason: string;
}

export async function detectHighlights(
  path: string,
  maxHighlights: number,
): Promise<Highlight[]> {
  return invoke<Highlight[]>("detect_highlights", { path, maxHighlights });
}

export async function transcribeAudio(
  path: string,
  languageHint?: string,
): Promise<SubtitleCue[]> {
  return invoke<SubtitleCue[]>("transcribe_audio", { path, languageHint });
}

export async function autoColorCorrect(
  path: string,
  timestampMs: number,
): Promise<ColorCorrection> {
  return invoke<ColorCorrection>("auto_color_correct", { path, timestampMs });
}

export async function suggestTransitions(
  path: string,
  fps: number,
): Promise<[number, TransitionSuggestion][]> {
  return invoke<[number, TransitionSuggestion][]>("suggest_transitions", { path, fps });
}

export interface ClipDescription {
  summary: string;
  tags: string[];
}

export async function describeClip(
  path: string,
  languageHint?: string,
): Promise<ClipDescription> {
  return invoke<ClipDescription>("describe_clip", { path, languageHint });
}

export async function refineSubtitles(
  cues: SubtitleCue[],
): Promise<SubtitleCue[]> {
  return invoke<SubtitleCue[]>("refine_subtitles", { cues });
}

export async function translateSubtitles(
  cues: SubtitleCue[],
  targetLanguage: string,
): Promise<SubtitleCue[]> {
  return invoke<SubtitleCue[]>("translate_subtitles", { cues, targetLanguage });
}

// Waveform extraction
export async function extractWaveform(
  path: string,
  peaksPerSecond: number,
): Promise<WaveformData> {
  return invoke<WaveformData>("extract_waveform", { path, peaksPerSecond });
}

// Thumbnail generation
export interface ThumbnailResult {
  timestamp_ms: number;
  data: string; // base64-encoded RGBA
}

export async function generateThumbnails(
  path: string,
  spec: ThumbnailSpec,
): Promise<ThumbnailResult[]> {
  return invoke<ThumbnailResult[]>("generate_thumbnails", { path, spec });
}

// Audio loudness measurement
export async function measureLoudness(path: string): Promise<number> {
  return invoke("measure_loudness", { path });
}

// Hardware detection
export async function detectHardware(): Promise<{
  accelerators: HardwareInfo[];
  available_encoders: ExportEncoder[];
}> {
  return invoke("detect_hardware");
}
