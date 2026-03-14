import { useCallback, useRef, useEffect } from "react";
import type { Clip } from "../../../types";
import { useProjectStore } from "../../../stores/projectStore";
import { useUIStore } from "../../../stores/uiStore";

export function useDragClip(trackId: string, clip: Clip, locked: boolean) {
  const moveClip = useProjectStore((s) => s.moveClip);
  const zoom = useUIStore((s) => s.zoom);
  const startX = useRef(0);
  const startFrame = useRef(0);
  const handlersRef = useRef<{
    move: ((e: MouseEvent) => void) | null;
    up: (() => void) | null;
  }>({ move: null, up: null });

  // Clean up drag listeners on unmount
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

      const handleMove = (e: MouseEvent) => {
        const dx = e.clientX - startX.current;
        const frameDelta = Math.round(dx / zoom);
        const newStart = Math.max(0, startFrame.current + frameDelta);
        moveClip(trackId, clip.id, newStart);
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
    [clip.id, clip.timeline_start, trackId, zoom, locked, moveClip],
  );

  return { onMouseDown };
}
