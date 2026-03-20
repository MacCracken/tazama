import { create } from "zustand";
import type { ActiveTool, ThumbnailStrategy } from "../types";

interface UIState {
  selectedClipId: string | null;
  selectedTrackId: string | null;
  zoom: number;
  scrollX: number;
  scrollY: number;
  activeTool: ActiveTool;
  thumbnailStrategy: ThumbnailStrategy;
  panelSizes: {
    mediaBrowser: number;
    inspector: number;
    timeline: number;
  };

  // Panel visibility
  showMixer: boolean;

  // Dialog visibility
  showExportDialog: boolean;
  showNewProjectDialog: boolean;

  // Toast
  toastMessage: string | null;
  toastType: "error" | "success" | "info";

  // Actions
  selectClip: (clipId: string | null) => void;
  selectTrack: (trackId: string | null) => void;
  setZoom: (zoom: number) => void;
  setScrollX: (x: number) => void;
  setScrollY: (y: number) => void;
  setActiveTool: (tool: ActiveTool) => void;
  setThumbnailStrategy: (strategy: ThumbnailStrategy) => void;
  setPanelSize: (panel: keyof UIState["panelSizes"], size: number) => void;
  toggleMixer: () => void;
  setShowExportDialog: (show: boolean) => void;
  setShowNewProjectDialog: (show: boolean) => void;
  showToast: (message: string, type?: "error" | "success" | "info") => void;
  clearToast: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  selectedClipId: null,
  selectedTrackId: null,
  zoom: 1,
  scrollX: 0,
  scrollY: 0,
  activeTool: "select",
  thumbnailStrategy: "SceneBased",
  panelSizes: {
    mediaBrowser: 240,
    inspector: 280,
    timeline: 40,
  },
  showMixer: false,
  showExportDialog: false,
  showNewProjectDialog: false,
  toastMessage: null,
  toastType: "info",

  selectClip: (clipId) => set({ selectedClipId: clipId }),
  selectTrack: (trackId) => set({ selectedTrackId: trackId }),
  setZoom: (zoom) => set({ zoom: Math.max(0.1, Math.min(10, zoom)) }),
  setScrollX: (x) => set({ scrollX: Math.max(0, x) }),
  setScrollY: (y) => set({ scrollY: Math.max(0, y) }),
  setActiveTool: (tool) => set({ activeTool: tool }),
  setThumbnailStrategy: (strategy) => set({ thumbnailStrategy: strategy }),
  setPanelSize: (panel, size) =>
    set((s) => ({
      panelSizes: { ...s.panelSizes, [panel]: size },
    })),
  toggleMixer: () => set((s) => ({ showMixer: !s.showMixer })),
  setShowExportDialog: (show) => set({ showExportDialog: show }),
  setShowNewProjectDialog: (show) => set({ showNewProjectDialog: show }),
  showToast: (message, type = "info") => set({ toastMessage: message, toastType: type }),
  clearToast: () => set({ toastMessage: null }),
}));
