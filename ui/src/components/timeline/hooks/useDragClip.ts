import { useCallback, useRef } from "react";
import type { Clip } from "../../../types";
import { useProjectStore } from "../../../stores/projectStore";
import { useUIStore } from "../../../stores/uiStore";

export function useDragClip(trackId: string, clip: Clip, locked: boolean) {
  const moveClip = useProjectStore((s) => s.moveClip);
  const zoom = useUIStore((s) => s.zoom);
  const startX = useRef(0);
  const startFrame = useRef(0);

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
      };

      document.addEventListener("mousemove", handleMove);
      document.addEventListener("mouseup", handleUp);
    },
    [clip.id, clip.timeline_start, trackId, zoom, locked, moveClip],
  );

  return { onMouseDown };
}
