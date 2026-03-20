import { useCallback, useEffect, useRef, useState } from "react";
import type { Clip, WaveformData } from "../../types";
import { useUIStore } from "../../stores/uiStore";
import { useDragClip } from "./hooks/useDragClip";
import { useTrimClip } from "./hooks/useTrimClip";
import { useRazorCut } from "./hooks/useRazorCut";
import { ClipContextMenu } from "./ClipContextMenu";
import * as commands from "../../ipc/commands";

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

// Module-level waveform cache
const waveformCache = new Map<string, WaveformData>();

function WaveformOverlay({
  waveform,
  width,
  height,
}: {
  waveform: WaveformData;
  width: number;
  height: number;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || waveform.peaks.length === 0) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);

    // Draw first channel (or mono)
    const peaks = waveform.peaks[0];
    if (!peaks || peaks.length === 0) return;

    const samplesPerPixel = peaks.length / width;
    const midY = height / 2;

    ctx.fillStyle = "rgba(255, 255, 255, 0.35)";
    ctx.beginPath();

    for (let x = 0; x < width; x++) {
      const peakIdx = Math.floor(x * samplesPerPixel);
      if (peakIdx >= peaks.length) break;
      const [min, max] = peaks[peakIdx];
      const top = midY - max * midY;
      const bottom = midY - min * midY;
      ctx.rect(x, top, 1, Math.max(bottom - top, 0.5));
    }

    ctx.fill();
  }, [waveform, width, height]);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 pointer-events-none"
      style={{ width, height }}
    />
  );
}

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
  const [waveform, setWaveform] = useState<WaveformData | null>(null);
  const [blockHeight, setBlockHeight] = useState(40);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  // Track container height for waveform canvas sizing
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setBlockHeight(entry.contentRect.height);
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const { onMouseDown: onDragStart } = useDragClip(trackId, clip, trackLocked);
  const { onMouseDownLeft, onMouseDownRight } = useTrimClip(
    trackId,
    clip,
    trackLocked,
  );
  const { onClick: onRazorClick } = useRazorCut(trackId, clip, zoom, scrollX);

  const left = clip.timeline_start * zoom - scrollX;
  const width = clip.duration * zoom;

  // Load waveform for audio/video clips
  useEffect(() => {
    const mediaPath = clip.media?.path;
    if (!mediaPath || (clip.kind !== "Audio" && clip.kind !== "Video")) return;

    if (waveformCache.has(mediaPath)) {
      setWaveform(waveformCache.get(mediaPath)!);
      return;
    }

    let cancelled = false;
    commands
      .extractWaveform(mediaPath, 100)
      .then((data) => {
        if (cancelled) return;
        waveformCache.set(mediaPath, data);
        setWaveform(data);
      })
      .catch(() => {});

    return () => { cancelled = true; };
  }, [clip.media?.path, clip.kind]);

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

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      selectClip(clip.id);
      setContextMenu({ x: e.clientX, y: e.clientY });
    },
    [clip.id, selectClip],
  );

  if (left + width < 0) return null;

  const blockWidth = Math.max(width, 4);

  return (
    <div
      ref={ref}
      className="absolute top-1 rounded cursor-pointer overflow-hidden"
      style={{
        left,
        width: blockWidth,
        height: "calc(100% - 8px)",
        background: clipColors[clip.kind] ?? "var(--clip-video)",
        opacity: clip.opacity,
        border: isSelected
          ? "2px solid var(--text-accent)"
          : "1px solid rgba(255,255,255,0.1)",
      }}
      onClick={handleClick}
      onMouseDown={handleMouseDown}
      onContextMenu={handleContextMenu}
    >
      {/* Waveform */}
      {waveform && blockHeight > 0 && (
        <WaveformOverlay
          waveform={waveform}
          width={blockWidth}
          height={blockHeight}
        />
      )}
      {/* Left trim handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-white/30 z-10"
        onMouseDown={(e) => {
          e.stopPropagation();
          if (!trackLocked) onMouseDownLeft(e);
        }}
      />
      {/* Clip label */}
      <div
        className="relative px-2 py-0.5 text-[10px] truncate pointer-events-none z-10"
        style={{ color: "rgba(255,255,255,0.9)" }}
      >
        {clip.name}
      </div>
      {/* Right trim handle */}
      <div
        className="absolute right-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-white/30 z-10"
        onMouseDown={(e) => {
          e.stopPropagation();
          if (!trackLocked) onMouseDownRight(e);
        }}
      />
      {/* Context menu */}
      {contextMenu && (
        <ClipContextMenu
          clip={clip}
          trackId={trackId}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
