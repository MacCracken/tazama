import type { Track } from "../../types";
import { TrackHeader } from "./TrackHeader";
import { ClipBlock } from "./ClipBlock";
import { useUIStore } from "../../stores/uiStore";

interface TrackRowProps {
  track: Track;
}

export function TrackRow({ track }: TrackRowProps) {
  const zoom = useUIStore((s) => s.zoom);
  const scrollX = useUIStore((s) => s.scrollX);

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
        style={{ background: "var(--bg-primary)" }}
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
