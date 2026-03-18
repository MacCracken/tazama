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

// Marker types
export type MarkerColor = "Red" | "Orange" | "Yellow" | "Green" | "Blue" | "Purple" | "White";

export interface Marker {
  id: string;
  name: string;
  frame: number;
  color: MarkerColor;
}

// Keyframe types
export type Interpolation =
  | "Linear"
  | "Hold"
  | { BezierCubic: { in_tangent: [number, number]; out_tangent: [number, number] } };

export interface Keyframe {
  id: string;
  frame: number;
  value: number;
  interpolation: Interpolation;
}

export interface KeyframeTrack {
  id: string;
  parameter: string;
  keyframes: Keyframe[];
}

// Multi-cam types
export interface MultiCamGroup {
  id: string;
  name: string;
  angles: [string, number][]; // [TrackId, sync_offset_frames]
}

// Timeline types
export interface Timeline {
  tracks: Track[];
  markers: Marker[];
  multicam_groups: MultiCamGroup[];
}

export interface Track {
  id: string;
  name: string;
  kind: TrackKind;
  clips: Clip[];
  muted: boolean;
  locked: boolean;
  solo: boolean;
  visible: boolean;
  volume: number;
  pan: number;
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
  proxy_path: string | null;
}

// Effect types - externally tagged enums matching Rust serde
export interface Effect {
  id: string;
  kind: EffectKind;
  enabled: boolean;
  keyframe_tracks: KeyframeTrack[];
}

export type EffectKind =
  | { ColorGrade: { brightness: number; contrast: number; saturation: number; temperature: number } }
  | { Crop: { left: number; top: number; right: number; bottom: number } }
  | { Speed: { factor: number } }
  | { Transition: { kind: TransitionKind; duration_frames: number } }
  | { FadeIn: { duration_frames: number } }
  | { FadeOut: { duration_frames: number } }
  | { Volume: { gain_db: number } }
  | { Eq: { low_gain_db: number; mid_gain_db: number; high_gain_db: number } }
  | { Compressor: { threshold_db: number; ratio: number; attack_ms: number; release_ms: number } }
  | { NoiseReduction: { strength: number } }
  | { Reverb: { room_size: number; damping: number; wet: number } }
  | { Lut: { lut_path: string } }
  | { Transform: { scale_x: number; scale_y: number; translate_x: number; translate_y: number } }
  | { Text: { content: string; font_family: string; font_size: number; color: [number, number, number, number]; x: number; y: number } }
  | { Plugin: { plugin_id: string; params: Record<string, number> } };

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
export type ExportFormat = "Mp4" | "WebM" | "ProRes" | "DnxHr" | "Mkv" | "Gif";

export interface ExportConfig {
  output_path: string;
  format: ExportFormat;
  width: number;
  height: number;
  frame_rate: [number, number];
  sample_rate: number;
  channels: number;
  hardware_accel: boolean;
}

export interface ExportProgress {
  frames_written: number;
  total_frames: number;
  done: boolean;
}

// Plugin types
export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  effects: PluginEffectDef[];
}

export interface PluginEffectDef {
  id: string;
  name: string;
  params: PluginParamDef[];
}

export interface PluginParamDef {
  name: string;
  default_value: number;
  min_value: number;
  max_value: number;
}

// UI-specific types
export type ActiveTool = "select" | "razor" | "slip";
