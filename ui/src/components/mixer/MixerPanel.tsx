import { useProjectStore } from "../../stores/projectStore";

export function MixerPanel() {
  const project = useProjectStore((s) => s.project);
  const setTrackVolume = useProjectStore((s) => s.setTrackVolume);
  const setTrackPan = useProjectStore((s) => s.setTrackPan);
  const toggleTrackMute = useProjectStore((s) => s.toggleTrackMute);
  const toggleTrackSolo = useProjectStore((s) => s.toggleTrackSolo);

  if (!project) return null;

  const tracks = project.timeline.tracks;

  if (tracks.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-[10px]" style={{ color: "var(--text-muted)" }}>
        No tracks
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-x-auto">
      {tracks.map((track) => (
        <div
          key={track.id}
          className="flex flex-col items-center flex-shrink-0 border-r px-2 py-1.5"
          style={{
            width: 72,
            borderColor: "var(--border-default)",
          }}
        >
          <span
            className="text-[10px] font-medium truncate w-full text-center mb-1"
            style={{ color: "var(--text-primary)" }}
          >
            {track.name}
          </span>

          {/* Volume fader — vertical */}
          <div className="flex flex-col items-center flex-1 min-h-0">
            <span className="text-[9px] font-mono" style={{ color: "var(--text-muted)" }}>
              {Math.round(track.volume * 100)}%
            </span>
            <input
              type="range"
              min={0}
              max={2}
              step={0.01}
              value={track.volume}
              onChange={(e) => setTrackVolume(track.id, parseFloat(e.target.value))}
              className="flex-1"
              style={{
                writingMode: "vertical-lr",
                direction: "rtl",
                width: 20,
              }}
            />
          </div>

          {/* Pan knob */}
          <div className="flex items-center gap-0.5 mt-1">
            <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>L</span>
            <input
              type="range"
              min={-1}
              max={1}
              step={0.01}
              value={track.pan}
              onChange={(e) => setTrackPan(track.id, parseFloat(e.target.value))}
              className="w-10 h-3"
            />
            <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>R</span>
          </div>

          {/* Mute / Solo */}
          <div className="flex gap-1 mt-1">
            <button
              onClick={() => toggleTrackMute(track.id)}
              className="text-[10px] w-5 h-5 rounded flex items-center justify-center font-bold"
              style={{
                background: track.muted ? "var(--error)" : "var(--bg-hover)",
                color: track.muted ? "#fff" : "var(--text-muted)",
              }}
            >
              M
            </button>
            <button
              onClick={() => toggleTrackSolo(track.id)}
              className="text-[10px] w-5 h-5 rounded flex items-center justify-center font-bold"
              style={{
                background: track.solo ? "var(--accent-primary)" : "var(--bg-hover)",
                color: track.solo ? "#fff" : "var(--text-muted)",
              }}
            >
              S
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
