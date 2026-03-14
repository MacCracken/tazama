import type { Track } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

interface TrackInspectorProps {
  track: Track;
}

export function TrackInspector({ track }: TrackInspectorProps) {
  const renameTrack = useProjectStore((s) => s.renameTrack);
  const toggleTrackMute = useProjectStore((s) => s.toggleTrackMute);
  const toggleTrackLock = useProjectStore((s) => s.toggleTrackLock);
  const removeTrack = useProjectStore((s) => s.removeTrack);

  return (
    <div className="flex flex-col h-full">
      <div
        className="px-2 py-1.5 border-b"
        style={{
          borderColor: "var(--border-default)",
          background: "var(--bg-tertiary)",
        }}
      >
        <span className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
          Track Inspector
        </span>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-3">
        <div>
          <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
            Name
          </label>
          <input
            type="text"
            value={track.name}
            onChange={(e) => renameTrack(track.id, e.target.value)}
            className="w-full px-1.5 py-1 rounded text-xs"
            style={{
              background: "var(--bg-primary)",
              border: "1px solid var(--border-default)",
            }}
          />
        </div>
        <div>
          <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
            Kind
          </label>
          <div className="text-xs" style={{ color: "var(--text-primary)" }}>
            {track.kind}
          </div>
        </div>
        <div className="flex items-center gap-2 text-xs">
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="checkbox"
              checked={track.muted}
              onChange={() => toggleTrackMute(track.id)}
            />
            <span style={{ color: "var(--text-secondary)" }}>Muted</span>
          </label>
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="checkbox"
              checked={track.locked}
              onChange={() => toggleTrackLock(track.id)}
            />
            <span style={{ color: "var(--text-secondary)" }}>Locked</span>
          </label>
        </div>
        <div>
          <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
            Clips
          </label>
          <div className="text-xs" style={{ color: "var(--text-primary)" }}>
            {track.clips.length}
          </div>
        </div>
        <button
          onClick={() => removeTrack(track.id)}
          className="w-full py-1 rounded text-xs"
          style={{
            background: "var(--bg-hover)",
            color: "var(--error)",
            border: "1px solid var(--border-default)",
          }}
        >
          Remove Track
        </button>
      </div>
    </div>
  );
}
