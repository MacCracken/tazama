import { invoke } from "@tauri-apps/api/core";
import type { Project, MediaInfo, ExportConfig } from "../types";

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
