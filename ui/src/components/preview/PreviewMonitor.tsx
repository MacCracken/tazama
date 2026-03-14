import { PreviewCanvas } from "./PreviewCanvas";
import { usePlaybackStore } from "../../stores/playbackStore";
import { useProjectStore } from "../../stores/projectStore";

export function PreviewMonitor() {
  const position = usePlaybackStore((s) => s.position);
  const project = useProjectStore((s) => s.project);
  const fps = project
    ? project.settings.frame_rate.numerator /
      project.settings.frame_rate.denominator
    : 30;

  const totalSeconds = position / fps;
  const m = Math.floor(totalSeconds / 60);
  const s = Math.floor(totalSeconds % 60);
  const f = Math.floor(position % fps);
  const timecode = `${m}:${String(s).padStart(2, "0")}:${String(f).padStart(2, "0")}`;

  return (
    <div className="flex flex-col items-center justify-center h-full p-4">
      <div className="relative w-full" style={{ maxWidth: 640 }}>
        <div style={{ paddingTop: "56.25%", position: "relative" }}>
          <PreviewCanvas />
        </div>
        <div
          className="absolute bottom-2 right-2 font-mono text-xs px-1.5 py-0.5 rounded"
          style={{
            background: "rgba(0,0,0,0.7)",
            color: "var(--text-secondary)",
          }}
        >
          {timecode}
        </div>
      </div>
    </div>
  );
}
