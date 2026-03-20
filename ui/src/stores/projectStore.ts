import { create } from "zustand";
import { produce } from "immer";
import type {
  Project,
  Track,
  TrackKind,
  Clip,
  Effect,
  Marker,
  MarkerColor,
  KeyframeTrack,
  MultiCamGroup,
} from "../types";
import * as commands from "../ipc/commands";
import { useHistoryStore } from "./historyStore";

interface MediaAsset {
  path: string;
  name: string;
  duration_frames: number;
}

interface ProjectState {
  project: Project | null;
  filePath: string | null;
  dirty: boolean;
  recentProjects: string[];
  mediaAssets: MediaAsset[];

  // Project lifecycle
  createProject: (name: string, width: number, height: number) => Promise<void>;
  openProject: (path: string) => Promise<void>;
  saveProject: () => Promise<void>;
  saveProjectAs: (path: string) => Promise<void>;
  closeProject: () => void;

  // Track operations
  addTrack: (name: string, kind: TrackKind) => void;
  removeTrack: (trackId: string) => void;
  toggleTrackMute: (trackId: string) => void;
  toggleTrackLock: (trackId: string) => void;
  renameTrack: (trackId: string, name: string) => void;
  setTrackVolume: (trackId: string, volume: number) => void;
  setTrackPan: (trackId: string, pan: number) => void;

  // Clip operations
  addClip: (trackId: string, clip: Clip) => void;
  removeClip: (trackId: string, clipId: string) => void;
  moveClip: (trackId: string, clipId: string, newStart: number) => void;
  splitClip: (trackId: string, clipId: string, frame: number) => void;
  trimClip: (
    trackId: string,
    clipId: string,
    newOffset: number,
    newDuration: number,
  ) => void;
  setClipOpacity: (trackId: string, clipId: string, opacity: number) => void;
  setClipVolume: (trackId: string, clipId: string, volume: number) => void;
  renameClip: (trackId: string, clipId: string, name: string) => void;

  // Marker operations
  addMarker: (name: string, frame: number, color: MarkerColor) => void;
  removeMarker: (markerId: string) => void;

  // Track solo/visible
  toggleTrackSolo: (trackId: string) => void;
  toggleTrackVisible: (trackId: string) => void;

  // Effect operations
  addEffect: (trackId: string, clipId: string, effect: Effect) => void;
  removeEffect: (
    trackId: string,
    clipId: string,
    effectId: string,
  ) => void;
  updateEffect: (
    trackId: string,
    clipId: string,
    effectId: string,
    kind: import("../types").EffectKind,
  ) => void;
  setKeyframeTracks: (
    trackId: string,
    clipId: string,
    effectId: string,
    tracks: KeyframeTrack[],
  ) => void;

  // Multi-cam operations
  createMultiCamGroup: (name: string, angles: [string, number][]) => void;
  removeMultiCamGroup: (groupId: string) => void;

  // Recording
  isRecording: boolean;
  startRecording: (sampleRate: number, channels: number) => Promise<void>;
  stopRecording: () => Promise<string | null>;

  // Media assets
  addMediaAsset: (asset: MediaAsset) => void;

  // Internal
  _mutate: (fn: (project: Project) => void) => void;
  /** Mutate without pushing undo — use for continuous drags/sliders. */
  _mutateSilent: (fn: (project: Project) => void) => void;
  /** Push the current state to the undo stack (call once before a silent-mutate sequence). */
  _pushUndo: () => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  project: null,
  filePath: null,
  dirty: false,
  recentProjects: [],
  mediaAssets: [],
  isRecording: false,

  createProject: async (name, width, height) => {
    const project = await commands.newProject(name, width, height);
    set({ project, filePath: null, dirty: false, mediaAssets: [] });
    useHistoryStore.getState().clear();
  },

  openProject: async (path) => {
    const project = await commands.openProject(path);
    const recent = get().recentProjects.filter((p) => p !== path);
    recent.unshift(path);
    set({
      project,
      filePath: path,
      dirty: false,
      recentProjects: recent.slice(0, 10),
      mediaAssets: [],
    });
    useHistoryStore.getState().clear();
  },

  saveProject: async () => {
    const { project, filePath } = get();
    if (!project || !filePath) return;
    await commands.saveProject(project, filePath);
    set({ dirty: false });
  },

  saveProjectAs: async (path) => {
    const { project } = get();
    if (!project) return;
    await commands.saveProject(project, path);
    const recent = get().recentProjects.filter((p) => p !== path);
    recent.unshift(path);
    set({ filePath: path, dirty: false, recentProjects: recent.slice(0, 10) });
  },

  closeProject: () => {
    set({ project: null, filePath: null, dirty: false, mediaAssets: [] });
    useHistoryStore.getState().clear();
  },

  _mutate: (fn) => {
    const { project } = get();
    if (!project) return;
    useHistoryStore.getState().pushState(project);
    set({
      project: produce(project, fn),
      dirty: true,
    });
  },

  _mutateSilent: (fn) => {
    const { project } = get();
    if (!project) return;
    set({
      project: produce(project, fn),
      dirty: true,
    });
  },

  _pushUndo: () => {
    const { project } = get();
    if (!project) return;
    useHistoryStore.getState().pushState(project);
  },

  addMarker: (name, frame, color) => {
    get()._mutate((p) => {
      const marker: Marker = {
        id: crypto.randomUUID(),
        name,
        frame,
        color,
      };
      p.timeline.markers.push(marker);
      p.timeline.markers.sort((a, b) => a.frame - b.frame);
    });
  },

  removeMarker: (markerId) => {
    get()._mutate((p) => {
      p.timeline.markers = p.timeline.markers.filter((m) => m.id !== markerId);
    });
  },

  toggleTrackSolo: (trackId) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.solo = !track.solo;
    });
  },

  toggleTrackVisible: (trackId) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.visible = !track.visible;
    });
  },

  addTrack: (name, kind) => {
    get()._mutate((p) => {
      const track: Track = {
        id: crypto.randomUUID(),
        name,
        kind,
        clips: [],
        muted: false,
        locked: false,
        solo: false,
        visible: true,
        volume: 1.0,
        pan: 0.0,
      };
      p.timeline.tracks.push(track);
    });
  },

  removeTrack: (trackId) => {
    get()._mutate((p) => {
      p.timeline.tracks = p.timeline.tracks.filter((t) => t.id !== trackId);
    });
  },

  toggleTrackMute: (trackId) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.muted = !track.muted;
    });
  },

  toggleTrackLock: (trackId) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.locked = !track.locked;
    });
  },

  renameTrack: (trackId, name) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.name = name;
    });
  },

  setTrackVolume: (trackId, volume) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.volume = volume;
    });
  },

  setTrackPan: (trackId, pan) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) track.pan = pan;
    });
  },

  addClip: (trackId, clip) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) {
        track.clips.push(clip);
        track.clips.sort((a, b) => a.timeline_start - b.timeline_start);
      }
    });
  },

  removeClip: (trackId, clipId) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) {
        track.clips = track.clips.filter((c) => c.id !== clipId);
      }
    });
  },

  moveClip: (trackId, clipId, newStart) => {
    get()._mutateSilent((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (track) {
        const clip = track.clips.find((c) => c.id === clipId);
        if (clip) {
          clip.timeline_start = newStart;
          track.clips.sort((a, b) => a.timeline_start - b.timeline_start);
        }
      }
    });
  },

  splitClip: (trackId, clipId, frame) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clipIdx = track.clips.findIndex((c) => c.id === clipId);
      if (clipIdx === -1) return;
      const clip = track.clips[clipIdx];
      if (frame <= clip.timeline_start || frame >= clip.timeline_start + clip.duration) return;

      const leftDuration = frame - clip.timeline_start;
      const rightDuration = clip.duration - leftDuration;
      const rightOffset = clip.source_offset + leftDuration;

      const rightClip: Clip = {
        ...structuredClone(clip),
        id: crypto.randomUUID(),
        timeline_start: frame,
        duration: rightDuration,
        source_offset: rightOffset,
      };

      clip.duration = leftDuration;
      track.clips.push(rightClip);
      track.clips.sort((a, b) => a.timeline_start - b.timeline_start);
    });
  },

  trimClip: (trackId, clipId, newOffset, newDuration) => {
    get()._mutateSilent((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (clip) {
        clip.source_offset = newOffset;
        clip.duration = newDuration;
      }
    });
  },

  setClipOpacity: (trackId, clipId, opacity) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (clip) clip.opacity = opacity;
    });
  },

  setClipVolume: (trackId, clipId, volume) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (clip) clip.volume = volume;
    });
  },

  renameClip: (trackId, clipId, name) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (clip) clip.name = name;
    });
  },

  addEffect: (trackId, clipId, effect) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (clip) clip.effects.push(effect);
    });
  },

  removeEffect: (trackId, clipId, effectId) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (clip) {
        clip.effects = clip.effects.filter((e) => e.id !== effectId);
      }
    });
  },

  updateEffect: (trackId, clipId, effectId, kind) => {
    get()._mutateSilent((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (!clip) return;
      const effect = clip.effects.find((e) => e.id === effectId);
      if (effect) effect.kind = kind;
    });
  },

  setKeyframeTracks: (trackId, clipId, effectId, tracks) => {
    get()._mutate((p) => {
      const track = p.timeline.tracks.find((t) => t.id === trackId);
      if (!track) return;
      const clip = track.clips.find((c) => c.id === clipId);
      if (!clip) return;
      const effect = clip.effects.find((e) => e.id === effectId);
      if (effect) effect.keyframe_tracks = tracks;
    });
  },

  createMultiCamGroup: (name, angles) => {
    get()._mutate((p) => {
      const group: MultiCamGroup = {
        id: crypto.randomUUID(),
        name,
        angles,
      };
      p.timeline.multicam_groups.push(group);
    });
  },

  removeMultiCamGroup: (groupId) => {
    get()._mutate((p) => {
      p.timeline.multicam_groups = p.timeline.multicam_groups.filter(
        (g) => g.id !== groupId,
      );
    });
  },

  startRecording: async (sampleRate, channels) => {
    await commands.startRecording(sampleRate, channels);
    set({ isRecording: true });
  },

  stopRecording: async () => {
    const path = await commands.stopRecording();
    set({ isRecording: false });
    return path;
  },

  addMediaAsset: (asset) => {
    set(
      produce((s: ProjectState) => {
        s.mediaAssets.push(asset);
      }),
    );
  },
}));
