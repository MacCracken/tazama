import { useCallback } from "react";
import { useUIStore } from "../../stores/uiStore";
import { usePlaybackStore } from "../../stores/playbackStore";
import { useProjectStore } from "../../stores/projectStore";
import type { MarkerColor } from "../../types";

const MARKER_COLORS: Record<MarkerColor, string> = {
  Red: "#ef4444",
  Orange: "#f97316",
  Yellow: "#eab308",
  Green: "#22c55e",
  Blue: "#3b82f6",
  Purple: "#a855f7",
  White: "#f5f5f5",
};

export function TimelineRuler() {
  const zoom = useUIStore((s) => s.zoom);
  const scrollX = useUIStore((s) => s.scrollX);
  const seek = usePlaybackStore((s) => s.seek);
  const project = useProjectStore((s) => s.project);
  const markers = project?.timeline.markers ?? [];

  const fps = project
    ? project.settings.frame_rate.numerator /
      project.settings.frame_rate.denominator
    : 30;

  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const rect = e.currentTarget.getBoundingClientRect();
      const x = e.clientX - rect.left + scrollX;
      const frame = Math.round(x / zoom);
      seek(Math.max(0, frame));
    },
    [zoom, scrollX, seek],
  );

  // Generate tick marks
  const tickInterval = Math.max(1, Math.round(fps / zoom));
  const majorInterval = tickInterval * 5;
  const totalWidth = 10000; // pixels
  const ticks: { x: number; major: boolean; label: string }[] = [];

  for (let frame = 0; frame < totalWidth / zoom; frame += tickInterval) {
    const x = frame * zoom - scrollX;
    if (x < -50 || x > totalWidth) continue;
    const isMajor = frame % majorInterval === 0;
    const seconds = frame / fps;
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    ticks.push({
      x,
      major: isMajor,
      label: isMajor ? `${m}:${String(s).padStart(2, "0")}` : "",
    });
  }

  return (
    <div
      className="relative flex-shrink-0 cursor-pointer border-b"
      style={{
        height: 24,
        background: "var(--bg-tertiary)",
        borderColor: "var(--border-default)",
        paddingLeft: "var(--track-header-width)",
      }}
      onClick={handleClick}
    >
      {ticks.map((tick, i) => (
        <div
          key={i}
          className="absolute top-0"
          style={{ left: tick.x }}
        >
          <div
            style={{
              width: 1,
              height: tick.major ? 16 : 8,
              background: tick.major
                ? "var(--text-secondary)"
                : "var(--text-muted)",
            }}
          />
          {tick.label && (
            <span
              className="absolute text-[10px] whitespace-nowrap"
              style={{
                top: 2,
                left: 4,
                color: "var(--text-secondary)",
              }}
            >
              {tick.label}
            </span>
          )}
        </div>
      ))}
      {markers.map((marker) => {
        const x = marker.frame * zoom - scrollX;
        if (x < -10 || x > totalWidth) return null;
        return (
          <div
            key={marker.id}
            className="absolute"
            style={{ left: x, top: 0 }}
            title={marker.name}
          >
            <div
              style={{
                width: 0,
                height: 0,
                borderLeft: "5px solid transparent",
                borderRight: "5px solid transparent",
                borderTop: `8px solid ${MARKER_COLORS[marker.color]}`,
                transform: "translateX(-5px)",
              }}
            />
          </div>
        );
      })}
    </div>
  );
}
