import { invoke } from "@tauri-apps/api/core";
import type { Project, MediaInfo, ExportConfig, HardwareInfo, ExportEncoder } from "../types";

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
