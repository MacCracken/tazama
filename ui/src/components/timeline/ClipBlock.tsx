import { useCallback, useRef } from "react";
import type { Clip } from "../../types";
import { useUIStore } from "../../stores/uiStore";
import { useDragClip } from "./hooks/useDragClip";
import { useTrimClip } from "./hooks/useTrimClip";
import { useRazorCut } from "./hooks/useRazorCut";

interface ClipBlockProps {
  clip: Clip;
  trackId: string;
  zoom: number;
  scrollX: number;
  trackLocked: boolean;
}

const clipColors: Record<string, string> = {
  Video: "var(--clip-video)",
  Audio: "var(--clip-audio)",
  Image: "var(--clip-image)",
  Title: "var(--clip-title)",
};

export function ClipBlock({
  clip,
  trackId,
  zoom,
  scrollX,
  trackLocked,
}: ClipBlockProps) {
  const selectClip = useUIStore((s) => s.selectClip);
  const selectedClipId = useUIStore((s) => s.selectedClipId);
  const activeTool = useUIStore((s) => s.activeTool);
  const isSelected = selectedClipId === clip.id;
  const ref = useRef<HTMLDivElement>(null);

  const { onMouseDown: onDragStart } = useDragClip(trackId, clip, trackLocked);
  const { onMouseDownLeft, onMouseDownRight } = useTrimClip(
    trackId,
    clip,
    trackLocked,
  );
  const { onClick: onRazorClick } = useRazorCut(trackId, clip, zoom, scrollX);

  const left = clip.timeline_start * zoom - scrollX;
  const width = clip.duration * zoom;

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      if (activeTool === "razor") {
        onRazorClick(e);
      } else {
        selectClip(clip.id);
      }
    },
    [activeTool, clip.id, selectClip, onRazorClick],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (activeTool === "select" && !trackLocked) {
        onDragStart(e);
      }
    },
    [activeTool, trackLocked, onDragStart],
  );

  if (left + width < 0) return null;

  return (
    <div
      ref={ref}
      className="absolute top-1 rounded cursor-pointer"
      style={{
        left,
        width: Math.max(width, 4),
        height: "calc(100% - 8px)",
        background: clipColors[clip.kind] ?? "var(--clip-video)",
        opacity: clip.opacity,
        border: isSelected
          ? "2px solid var(--text-accent)"
          : "1px solid rgba(255,255,255,0.1)",
      }}
      onClick={handleClick}
      onMouseDown={handleMouseDown}
    >
      {/* Left trim handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-white/30"
        onMouseDown={(e) => {
          e.stopPropagation();
          if (!trackLocked) onMouseDownLeft(e);
        }}
      />
      {/* Clip label */}
      <div
        className="px-2 py-0.5 text-[10px] truncate pointer-events-none"
        style={{ color: "rgba(255,255,255,0.9)" }}
      >
        {clip.name}
      </div>
      {/* Right trim handle */}
      <div
        className="absolute right-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-white/30"
        onMouseDown={(e) => {
          e.stopPropagation();
          if (!trackLocked) onMouseDownRight(e);
        }}
      />
    </div>
  );
}
