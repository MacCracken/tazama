import { useCallback, useRef, useEffect } from "react";
import type { Clip } from "../../../types";
import { useProjectStore } from "../../../stores/projectStore";
import { useUIStore } from "../../../stores/uiStore";
import { useSnap } from "./useSnap";

export function useDragClip(trackId: string, clip: Clip, locked: boolean) {
  const moveClip = useProjectStore((s) => s.moveClip);
  const pushUndo = useProjectStore((s) => s._pushUndo);
  const zoom = useUIStore((s) => s.zoom);
  const snap = useSnap();
  const startX = useRef(0);
  const startFrame = useRef(0);
  const pushed = useRef(false);
  const handlersRef = useRef<{
    move: ((e: MouseEvent) => void) | null;
    up: (() => void) | null;
  }>({ move: null, up: null });

  useEffect(() => {
    return () => {
      if (handlersRef.current.move) {
        document.removeEventListener("mousemove", handlersRef.current.move);
      }
      if (handlersRef.current.up) {
        document.removeEventListener("mouseup", handlersRef.current.up);
      }
    };
  }, []);

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (locked || e.button !== 0) return;
      e.preventDefault();
      startX.current = e.clientX;
      startFrame.current = clip.timeline_start;
      pushed.current = false;

      const handleMove = (e: MouseEvent) => {
        if (!pushed.current) {
          pushUndo();
          pushed.current = true;
        }
        const dx = e.clientX - startX.current;
        const frameDelta = Math.round(dx / zoom);
        const raw = Math.max(0, startFrame.current + frameDelta);
        const snapped = snap(raw, clip.id);
        moveClip(trackId, clip.id, snapped);
      };

      const handleUp = () => {
        document.removeEventListener("mousemove", handleMove);
        document.removeEventListener("mouseup", handleUp);
        handlersRef.current = { move: null, up: null };
      };

      handlersRef.current = { move: handleMove, up: handleUp };
      document.addEventListener("mousemove", handleMove);
      document.addEventListener("mouseup", handleUp);
    },
    [clip.id, clip.timeline_start, trackId, zoom, locked, moveClip, pushUndo, snap],
  );

  return { onMouseDown };
}
