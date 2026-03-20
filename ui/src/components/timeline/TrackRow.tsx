import { useCallback, useState } from "react";
import type { Track, Clip, ClipKind } from "../../types";
import { TrackHeader } from "./TrackHeader";
import { ClipBlock } from "./ClipBlock";
import { useUIStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";

interface TrackRowProps {
  track: Track;
}

export function TrackRow({ track }: TrackRowProps) {
  const zoom = useUIStore((s) => s.zoom);
  const scrollX = useUIStore((s) => s.scrollX);
  const addClip = useProjectStore((s) => s.addClip);
  const [dropHighlight, setDropHighlight] = useState(false);

  const handleDragOver = useCallback(
    (e: React.DragEvent) => {
      if (track.locked) return;
      if (!e.dataTransfer.types.includes("application/tazama-media")) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      setDropHighlight(true);
    },
    [track.locked],
  );

  const handleDragLeave = useCallback(() => {
    setDropHighlight(false);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      setDropHighlight(false);
      if (track.locked) return;

      const raw = e.dataTransfer.getData("application/tazama-media");
      if (!raw) return;

      e.preventDefault();
      const asset = JSON.parse(raw) as { path: string; name: string; duration_frames: number };

      // Calculate drop position in frames from mouse position
      const rect = e.currentTarget.getBoundingClientRect();
      const localX = e.clientX - rect.left;
      const dropFrame = Math.max(0, Math.round((localX + scrollX) / zoom));

      const kind: ClipKind = track.kind === "Audio" ? "Audio" : "Video";

      const clip: Clip = {
        id: crypto.randomUUID(),
        name: asset.name,
        kind,
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
        timeline_start: dropFrame,
        duration: asset.duration_frames,
        source_offset: 0,
        effects: [],
        opacity: 1.0,
        volume: 1.0,
      };

      addClip(track.id, clip);
    },
    [track.id, track.kind, track.locked, zoom, scrollX, addClip],
  );

  return (
    <div
      className="flex border-b"
      style={{
        borderColor: "var(--border-subtle)",
        minHeight: 48,
        opacity: track.muted ? 0.5 : 1,
      }}
    >
      <TrackHeader track={track} />
      <div
        className="relative flex-1 overflow-hidden"
        style={{
          background: dropHighlight
            ? "rgba(var(--accent-primary-rgb, 59, 130, 246), 0.1)"
            : "var(--bg-primary)",
          outline: dropHighlight ? "2px dashed var(--text-accent)" : "none",
          outlineOffset: -2,
        }}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {track.clips.map((clip) => (
          <ClipBlock
            key={clip.id}
            clip={clip}
            trackId={track.id}
            zoom={zoom}
            scrollX={scrollX}
            trackLocked={track.locked}
          />
        ))}
      </div>
    </div>
  );
}
