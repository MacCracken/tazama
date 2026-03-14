import { create } from "zustand";
import type { Project } from "../types";
import { useProjectStore } from "./projectStore";

const MAX_HISTORY = 100;

interface HistoryState {
  undoStack: Project[];
  redoStack: Project[];
  pushState: (project: Project) => void;
  undo: () => void;
  redo: () => void;
  clear: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  undoStack: [],
  redoStack: [],

  pushState: (project) => {
    set((s) => ({
      undoStack: [...s.undoStack.slice(-(MAX_HISTORY - 1)), structuredClone(project)],
      redoStack: [],
    }));
  },

  undo: () => {
    const { undoStack } = get();
    if (undoStack.length === 0) return;

    const current = useProjectStore.getState().project;
    if (!current) return;

    const previous = undoStack[undoStack.length - 1];
    set((s) => ({
      undoStack: s.undoStack.slice(0, -1),
      redoStack: [...s.redoStack, structuredClone(current)],
    }));
    useProjectStore.setState({ project: previous, dirty: true });
  },

  redo: () => {
    const { redoStack } = get();
    if (redoStack.length === 0) return;

    const current = useProjectStore.getState().project;
    if (!current) return;

    const next = redoStack[redoStack.length - 1];
    set((s) => ({
      redoStack: s.redoStack.slice(0, -1),
      undoStack: [...s.undoStack, structuredClone(current)],
    }));
    useProjectStore.setState({ project: next, dirty: true });
  },

  clear: () => {
    set({ undoStack: [], redoStack: [] });
  },

  canUndo: () => get().undoStack.length > 0,
  canRedo: () => get().redoStack.length > 0,
}));
