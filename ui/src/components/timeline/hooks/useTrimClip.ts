import { useCallback, useRef } from "react";
import type { Clip } from "../../../types";
import { useProjectStore } from "../../../stores/projectStore";
import { useUIStore } from "../../../stores/uiStore";

export function useTrimClip(trackId: string, clip: Clip, locked: boolean) {
  const trimClip = useProjectStore((s) => s.trimClip);
  const zoom = useUIStore((s) => s.zoom);
  const startX = useRef(0);
  const origOffset = useRef(0);
  const origDuration = useRef(0);
  const origStart = useRef(0);

  const onMouseDownLeft = useCallback(
    (e: React.MouseEvent) => {
      if (locked || e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      startX.current = e.clientX;
      origOffset.current = clip.source_offset;
      origDuration.current = clip.duration;
      origStart.current = clip.timeline_start;

      const handleMove = (e: MouseEvent) => {
        const dx = e.clientX - startX.current;
        const frameDelta = Math.round(dx / zoom);
        const newOffset = Math.max(0, origOffset.current + frameDelta);
        const newDuration = Math.max(1, origDuration.current - frameDelta);
        trimClip(trackId, clip.id, newOffset, newDuration);
      };

      const handleUp = () => {
        document.removeEventListener("mousemove", handleMove);
        document.removeEventListener("mouseup", handleUp);
      };

      document.addEventListener("mousemove", handleMove);
      document.addEventListener("mouseup", handleUp);
    },
    [clip, trackId, zoom, locked, trimClip],
  );

  const onMouseDownRight = useCallback(
    (e: React.MouseEvent) => {
      if (locked || e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      startX.current = e.clientX;
      origDuration.current = clip.duration;

      const handleMove = (e: MouseEvent) => {
        const dx = e.clientX - startX.current;
        const frameDelta = Math.round(dx / zoom);
        const newDuration = Math.max(1, origDuration.current + frameDelta);
        trimClip(trackId, clip.id, clip.source_offset, newDuration);
      };

      const handleUp = () => {
        document.removeEventListener("mousemove", handleMove);
        document.removeEventListener("mouseup", handleUp);
      };

      document.addEventListener("mousemove", handleMove);
      document.addEventListener("mouseup", handleUp);
    },
    [clip, trackId, zoom, locked, trimClip],
  );

  return { onMouseDownLeft, onMouseDownRight };
}
