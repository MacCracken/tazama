import { useCallback, useRef, useEffect } from "react";
import type { Clip } from "../../../types";
import { useProjectStore } from "../../../stores/projectStore";
import { useUIStore } from "../../../stores/uiStore";
import { useSnap } from "./useSnap";

export function useTrimClip(trackId: string, clip: Clip, locked: boolean) {
  const trimClip = useProjectStore((s) => s.trimClip);
  const pushUndo = useProjectStore((s) => s._pushUndo);
  const zoom = useUIStore((s) => s.zoom);
  const snap = useSnap();
  const startX = useRef(0);
  const origOffset = useRef(0);
  const origDuration = useRef(0);
  const origStart = useRef(0);
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

  const onMouseDownLeft = useCallback(
    (e: React.MouseEvent) => {
      if (locked || e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      startX.current = e.clientX;
      origOffset.current = clip.source_offset;
      origDuration.current = clip.duration;
      origStart.current = clip.timeline_start;
      pushed.current = false;

      const handleMove = (e: MouseEvent) => {
        if (!pushed.current) {
          pushUndo();
          pushed.current = true;
        }
        const dx = e.clientX - startX.current;
        const frameDelta = Math.round(dx / zoom);

        // Snap the left edge
        const rawLeftEdge = origStart.current + frameDelta;
        const snappedLeftEdge = snap(rawLeftEdge, clip.id);
        const actualDelta = snappedLeftEdge - origStart.current;

        const newOffset = Math.max(0, origOffset.current + actualDelta);
        const newDuration = Math.max(1, origDuration.current - actualDelta);
        trimClip(trackId, clip.id, newOffset, newDuration);
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
    [clip, trackId, zoom, locked, trimClip, pushUndo, snap],
  );

  const onMouseDownRight = useCallback(
    (e: React.MouseEvent) => {
      if (locked || e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      startX.current = e.clientX;
      origDuration.current = clip.duration;
      pushed.current = false;

      const handleMove = (e: MouseEvent) => {
        if (!pushed.current) {
          pushUndo();
          pushed.current = true;
        }
        const dx = e.clientX - startX.current;
        const frameDelta = Math.round(dx / zoom);

        // Snap the right edge
        const rawRightEdge = clip.timeline_start + origDuration.current + frameDelta;
        const snappedRightEdge = snap(rawRightEdge, clip.id);
        const newDuration = Math.max(1, snappedRightEdge - clip.timeline_start);
        trimClip(trackId, clip.id, clip.source_offset, newDuration);
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
    [clip, trackId, zoom, locked, trimClip, pushUndo, snap],
  );

  return { onMouseDownLeft, onMouseDownRight };
}
