import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { usePlaybackStore } from "../stores/playbackStore";
import { useUIStore } from "../stores/uiStore";
import { useHistoryStore } from "../stores/historyStore";
import { useProjectStore } from "../stores/projectStore";

export function useKeyboardShortcuts() {
  const project = useProjectStore((s) => s.project);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't handle shortcuts when typing in inputs
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const ctrl = e.ctrlKey || e.metaKey;

      // Transport
      if (e.key === " ") {
        e.preventDefault();
        usePlaybackStore.getState().togglePlayPause();
        return;
      }
      if (e.key === "ArrowRight" && !ctrl) {
        e.preventDefault();
        usePlaybackStore.getState().stepForward();
        return;
      }
      if (e.key === "ArrowLeft" && !ctrl) {
        e.preventDefault();
        usePlaybackStore.getState().stepBack();
        return;
      }

      // J/K/L shuttle
      if (e.key === "j") {
        const pb = usePlaybackStore.getState();
        const speed = Math.max(-4, pb.shuttleSpeed - 1);
        pb.setShuttleSpeed(speed);
        if (!pb.isPlaying) pb.play();
        return;
      }
      if (e.key === "k") {
        usePlaybackStore.getState().pause();
        return;
      }
      if (e.key === "l") {
        const pb = usePlaybackStore.getState();
        const speed = Math.min(4, pb.shuttleSpeed + 1);
        pb.setShuttleSpeed(speed);
        if (!pb.isPlaying) pb.play();
        return;
      }

      // Loop points
      if (e.key === "i") {
        const pb = usePlaybackStore.getState();
        pb.setLoopRegion(pb.position, pb.loopOut);
        return;
      }
      if (e.key === "o") {
        const pb = usePlaybackStore.getState();
        pb.setLoopRegion(pb.loopIn, pb.position);
        return;
      }

      // Edit tools
      if (e.key === "v" && !ctrl) {
        useUIStore.getState().setActiveTool("select");
        return;
      }
      if (e.key === "b" && !ctrl) {
        useUIStore.getState().setActiveTool("razor");
        return;
      }
      if (e.key === "s" && !ctrl) {
        useUIStore.getState().setActiveTool("slip");
        return;
      }

      // Delete selected clip
      if (e.key === "Delete" || e.key === "Backspace") {
        const ui = useUIStore.getState();
        const proj = useProjectStore.getState().project;
        if (ui.selectedClipId && proj) {
          for (const track of proj.timeline.tracks) {
            if (track.clips.some((c) => c.id === ui.selectedClipId)) {
              useProjectStore.getState().removeClip(track.id, ui.selectedClipId);
              ui.selectClip(null);
              break;
            }
          }
        }
        return;
      }

      // Zoom
      if (e.key === "=" || e.key === "+") {
        const ui = useUIStore.getState();
        ui.setZoom(ui.zoom + 0.2);
        return;
      }
      if (e.key === "-") {
        const ui = useUIStore.getState();
        ui.setZoom(ui.zoom - 0.2);
        return;
      }

      // Ctrl shortcuts
      if (ctrl) {
        if (e.key === "z" && !e.shiftKey) {
          e.preventDefault();
          useHistoryStore.getState().undo();
          return;
        }
        if ((e.key === "z" && e.shiftKey) || e.key === "y") {
          e.preventDefault();
          useHistoryStore.getState().redo();
          return;
        }
        if (e.key === "s") {
          e.preventDefault();
          useProjectStore.getState().saveProject();
          return;
        }
        if (e.key === "n") {
          e.preventDefault();
          useUIStore.getState().setShowNewProjectDialog(true);
          return;
        }
        if (e.key === "e") {
          e.preventDefault();
          if (project) useUIStore.getState().setShowExportDialog(true);
          return;
        }
        if (e.key === "o") {
          e.preventDefault();
          open({
            filters: [{ name: "Tazama Project", extensions: ["tazama"] }],
          }).then((selected) => {
            if (selected) useProjectStore.getState().openProject(selected);
          });
          return;
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [project]);
}
