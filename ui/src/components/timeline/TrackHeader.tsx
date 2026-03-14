import type { Track } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";

interface TrackHeaderProps {
  track: Track;
}

export function TrackHeader({ track }: TrackHeaderProps) {
  const toggleMute = useProjectStore((s) => s.toggleTrackMute);
  const toggleLock = useProjectStore((s) => s.toggleTrackLock);
  const toggleSolo = useProjectStore((s) => s.toggleTrackSolo);
  const toggleVisible = useProjectStore((s) => s.toggleTrackVisible);
  const selectTrack = useUIStore((s) => s.selectTrack);
  const selectedTrackId = useUIStore((s) => s.selectedTrackId);
  const isSelected = selectedTrackId === track.id;

  return (
    <div
      className="flex items-center gap-1 px-2 flex-shrink-0 cursor-pointer"
      style={{
        width: "var(--track-header-width)",
        background: isSelected ? "var(--bg-active)" : "var(--bg-secondary)",
        borderRight: "1px solid var(--border-default)",
      }}
      onClick={() => selectTrack(track.id)}
    >
      <span
        className="text-[10px] font-medium rounded px-1"
        style={{
          background:
            track.kind === "Video"
              ? "var(--clip-video)"
              : "var(--clip-audio)",
          color: "#fff",
        }}
      >
        {track.kind === "Video" ? "V" : "A"}
      </span>
      <span
        className="flex-1 text-xs truncate"
        style={{ color: "var(--text-primary)" }}
      >
        {track.name}
      </span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          toggleSolo(track.id);
        }}
        className="text-[10px] px-1 rounded"
        style={{
          color: track.solo ? "var(--accent)" : "var(--text-muted)",
        }}
        title={track.solo ? "Unsolo" : "Solo"}
      >
        S
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          toggleMute(track.id);
        }}
        className="text-[10px] px-1 rounded"
        style={{
          color: track.muted ? "var(--warning)" : "var(--text-muted)",
        }}
        title={track.muted ? "Unmute" : "Mute"}
      >
        M
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          toggleVisible(track.id);
        }}
        className="text-[10px] px-1 rounded"
        style={{
          color: track.visible ? "var(--text-muted)" : "var(--error)",
        }}
        title={track.visible ? "Hide" : "Show"}
      >
        {track.visible ? "\u{1F441}" : "\u{1F441}\u{200D}\u{1F5E8}"}
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          toggleLock(track.id);
        }}
        className="text-[10px] px-1 rounded"
        style={{
          color: track.locked ? "var(--error)" : "var(--text-muted)",
        }}
        title={track.locked ? "Unlock" : "Lock"}
      >
        L
      </button>
    </div>
  );
}
