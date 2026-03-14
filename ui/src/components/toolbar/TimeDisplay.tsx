import { usePlaybackStore } from "../../stores/playbackStore";
import { useProjectStore } from "../../stores/projectStore";

function formatTimecode(frame: number, fps: number): string {
  if (fps <= 0) return "00:00:00:00";
  const totalSeconds = frame / fps;
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = Math.floor(totalSeconds % 60);
  const f = Math.floor(frame % fps);
  return [
    String(h).padStart(2, "0"),
    String(m).padStart(2, "0"),
    String(s).padStart(2, "0"),
    String(f).padStart(2, "0"),
  ].join(":");
}

export function TimeDisplay() {
  const position = usePlaybackStore((s) => s.position);
  const project = useProjectStore((s) => s.project);
  const fps = project
    ? project.settings.frame_rate.numerator /
      project.settings.frame_rate.denominator
    : 30;

  return (
    <div
      className="font-mono text-sm px-3 py-1 rounded"
      style={{
        background: "var(--bg-primary)",
        color: "var(--text-primary)",
        minWidth: 110,
        textAlign: "center",
      }}
    >
      {formatTimecode(position, fps)}
    </div>
  );
}
