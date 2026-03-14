import { create } from "zustand";

interface PlaybackState {
  position: number;
  isPlaying: boolean;
  shuttleSpeed: number;
  loopIn: number | null;
  loopOut: number | null;

  play: () => void;
  pause: () => void;
  stop: () => void;
  togglePlayPause: () => void;
  seek: (frame: number) => void;
  stepForward: () => void;
  stepBack: () => void;
  setShuttleSpeed: (speed: number) => void;
  setLoopRegion: (inPoint: number | null, outPoint: number | null) => void;
}

export const usePlaybackStore = create<PlaybackState>((set) => ({
  position: 0,
  isPlaying: false,
  shuttleSpeed: 1,
  loopIn: null,
  loopOut: null,

  play: () => set({ isPlaying: true }),
  pause: () => set({ isPlaying: false }),
  stop: () => set({ isPlaying: false, position: 0, shuttleSpeed: 1 }),
  togglePlayPause: () => set((s) => ({ isPlaying: !s.isPlaying })),

  seek: (frame) => set({ position: Math.max(0, frame) }),
  stepForward: () => set((s) => ({ position: s.position + 1 })),
  stepBack: () => set((s) => ({ position: Math.max(0, s.position - 1) })),

  setShuttleSpeed: (speed) => set({ shuttleSpeed: speed }),

  setLoopRegion: (inPoint, outPoint) =>
    set({ loopIn: inPoint, loopOut: outPoint }),
}));
