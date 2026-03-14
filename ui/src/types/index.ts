// Project types
export interface Project {
  id: string;
  name: string;
  settings: ProjectSettings;
  timeline: Timeline;
  created_at: string;
  modified_at: string;
}

export interface ProjectSettings {
  width: number;
  height: number;
  frame_rate: FrameRate;
  sample_rate: number;
  channels: number;
}

export interface FrameRate {
  numerator: number;
  denominator: number;
}

// Timeline types
export interface Timeline {
  tracks: Track[];
}

export interface Track {
  id: string;
  name: string;
  kind: TrackKind;
  clips: Clip[];
  muted: boolean;
  locked: boolean;
}

export type TrackKind = "Video" | "Audio";

export interface Clip {
  id: string;
  name: string;
  kind: ClipKind;
  media: MediaRef | null;
  timeline_start: number;
  duration: number;
  source_offset: number;
  effects: Effect[];
  opacity: number;
  volume: number;
}

export type ClipKind = "Video" | "Audio" | "Image" | "Title";

export interface MediaRef {
  path: string;
  duration_frames: number;
  width: number | null;
  height: number | null;
  sample_rate: number | null;
  channels: number | null;
  info: MediaInfo | null;
}

// Effect types - externally tagged enums matching Rust serde
export interface Effect {
  id: string;
  kind: EffectKind;
  enabled: boolean;
}

export type EffectKind =
  | { ColorGrade: { brightness: number; contrast: number; saturation: number; temperature: number } }
  | { Crop: { left: number; top: number; right: number; bottom: number } }
  | { Speed: { factor: number } }
  | { Transition: { kind: TransitionKind; duration_frames: number } }
  | { FadeIn: { duration_frames: number } }
  | { FadeOut: { duration_frames: number } }
  | { Volume: { gain_db: number } };

export type TransitionKind = "Cut" | "Dissolve" | "Wipe" | "Fade";

// Playback types
export type PlaybackState = "Stopped" | "Playing" | "Paused";

// Media info types
export type Codec = "H264" | "H265" | "Vp9" | "Av1" | "Aac" | "Opus" | "Flac" | "Mp3" | "Other";

export type ContainerFormat = "Mp4" | "Mkv" | "WebM" | "Mov" | "Avi" | "Other";

export interface VideoStreamInfo {
  codec: Codec;
  width: number;
  height: number;
  frame_rate: [number, number];
  bit_depth: number;
  pixel_format: string;
}

export interface AudioStreamInfo {
  codec: Codec;
  sample_rate: number;
  channels: number;
  bit_depth: number;
}

export interface MediaInfo {
  duration_ms: number;
  duration_frames: number;
  container: ContainerFormat;
  video_streams: VideoStreamInfo[];
  audio_streams: AudioStreamInfo[];
  file_size: number;
}

// Export types
export type ExportFormat = "Mp4" | "WebM";

export interface ExportConfig {
  output_path: string;
  format: ExportFormat;
  width: number;
  height: number;
  frame_rate: [number, number];
  sample_rate: number;
  channels: number;
}

export interface ExportProgress {
  frames_written: number;
  total_frames: number;
  done: boolean;
}

// UI-specific types
export type ActiveTool = "select" | "razor" | "slip";
