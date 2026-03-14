import { usePlaybackStore } from "../../stores/playbackStore";
import { useUIStore } from "../../stores/uiStore";

export function Playhead() {
  const position = usePlaybackStore((s) => s.position);
  const zoom = useUIStore((s) => s.zoom);
  const scrollX = useUIStore((s) => s.scrollX);
  const left = position * zoom - scrollX;

  return (
    <div
      className="absolute top-0 bottom-0 pointer-events-none z-10"
      style={{
        left: `calc(var(--track-header-width) + ${left}px)`,
        width: 1,
        background: "var(--playhead)",
      }}
    >
      <div
        className="absolute -top-0.5 -left-[5px]"
        style={{
          width: 0,
          height: 0,
          borderLeft: "5px solid transparent",
          borderRight: "5px solid transparent",
          borderTop: "6px solid var(--playhead)",
        }}
      />
    </div>
  );
}
