import { useCallback, useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import type { Clip, ClipKind } from "../../types";
import * as commands from "../../ipc/commands";

interface MediaAsset {
  path: string;
  name: string;
  duration_frames: number;
}

interface MediaItemProps {
  asset: MediaAsset;
}

// Module-level cache so thumbnails survive re-renders
const thumbCache = new Map<string, string>();

export function MediaItem({ asset }: MediaItemProps) {
  const addClip = useProjectStore((s) => s.addClip);
  const project = useProjectStore((s) => s.project);
  const thumbnailStrategy = useUIStore((s) => s.thumbnailStrategy);
  const [thumb, setThumb] = useState<string | null>(thumbCache.get(asset.path) ?? null);

  useEffect(() => {
    if (thumbCache.has(asset.path)) {
      setThumb(thumbCache.get(asset.path)!);
      return;
    }

    let cancelled = false;
    commands
      .generateThumbnails(asset.path, {
        width: 128,
        height: 72,
        interval_ms: Math.max(1000, Math.floor((asset.duration_frames / 30) * 1000)),
        strategy: thumbnailStrategy,
      })
      .then((results) => {
        if (cancelled || results.length === 0) return;
        const dataUrl = `data:image/raw;base64,${results[0].data}`;
        thumbCache.set(asset.path, dataUrl);
        setThumb(dataUrl);
      })
      .catch(() => {});

    return () => { cancelled = true; };
  }, [asset.path, asset.duration_frames, thumbnailStrategy]);

  const handleDragStart = useCallback(
    (e: React.DragEvent) => {
      e.dataTransfer.setData("application/tazama-media", JSON.stringify(asset));
      e.dataTransfer.effectAllowed = "copy";
    },
    [asset],
  );

  const handleDoubleClick = useCallback(() => {
    if (!project) return;
    const tracks = project.timeline.tracks;
    const videoTrack = tracks.find((t) => t.kind === "Video");
    if (!videoTrack) {
      useUIStore.getState().showToast("No video track found. Add a track first.", "error");
      return;
    }

    const clip: Clip = {
      id: crypto.randomUUID(),
      name: asset.name,
      kind: "Video" as ClipKind,
      media: {
        path: asset.path,
        duration_frames: asset.duration_frames,
        width: null,
        height: null,
        sample_rate: null,
        channels: null,
        info: null,
        proxy_path: null,
      },
      timeline_start: 0,
      duration: asset.duration_frames,
      source_offset: 0,
      effects: [],
      opacity: 1.0,
      volume: 1.0,
    };

    addClip(videoTrack.id, clip);
  }, [asset, project, addClip]);

  return (
    <div
      className="flex items-center gap-2 px-2 py-1 rounded cursor-grab hover:bg-[var(--bg-hover)] text-xs"
      draggable
      onDragStart={handleDragStart}
      onDoubleClick={handleDoubleClick}
      style={{ color: "var(--text-primary)" }}
    >
      {thumb ? (
        <div
          className="flex-shrink-0 rounded overflow-hidden"
          style={{ width: 32, height: 18, background: "var(--bg-primary)" }}
        >
          <img
            src={thumb}
            alt=""
            className="w-full h-full object-cover"
            style={{ imageRendering: "auto" }}
          />
        </div>
      ) : (
        <svg width="14" height="14" viewBox="0 0 14 14" fill="var(--text-muted)">
          <rect x="1" y="1" width="12" height="12" rx="2" stroke="currentColor" fill="none" strokeWidth="1" />
          <path d="M5 4l4 3-4 3V4z" fill="currentColor" />
        </svg>
      )}
      <span className="flex-1 truncate">{asset.name}</span>
    </div>
  );
}
