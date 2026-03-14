import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ExportProgress as ExportProgressType } from "../../types";

export function ExportProgress() {
  const [progress, setProgress] = useState<ExportProgressType>({
    frames_written: 0,
    total_frames: 0,
    done: false,
  });

  useEffect(() => {
    const unlisten = listen<ExportProgressType>("export-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const pct =
    progress.total_frames > 0
      ? Math.round((progress.frames_written / progress.total_frames) * 100)
      : 0;

  return (
    <div className="space-y-2">
      <div className="text-xs" style={{ color: "var(--text-secondary)" }}>
        Exporting... {pct}%
      </div>
      <div
        className="h-2 rounded-full overflow-hidden"
        style={{ background: "var(--bg-primary)" }}
      >
        <div
          className="h-full rounded-full transition-all duration-200"
          style={{
            width: `${pct}%`,
            background: "var(--accent-primary)",
          }}
        />
      </div>
      <div className="text-[10px]" style={{ color: "var(--text-muted)" }}>
        {progress.frames_written} / {progress.total_frames} frames
      </div>
    </div>
  );
}
