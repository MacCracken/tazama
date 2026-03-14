import { useRef, useCallback } from "react";
import { TimelineRuler } from "./TimelineRuler";
import { TrackList } from "./TrackList";
import { Playhead } from "./Playhead";
import { useUIStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";

export function TimelinePanel() {
  const containerRef = useRef<HTMLDivElement>(null);
  const zoom = useUIStore((s) => s.zoom);
  const setZoom = useUIStore((s) => s.setZoom);
  const scrollX = useUIStore((s) => s.scrollX);
  const setScrollX = useUIStore((s) => s.setScrollX);
  const project = useProjectStore((s) => s.project);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const delta = e.deltaY > 0 ? -0.1 : 0.1;
        setZoom(zoom + delta);
      } else {
        setScrollX(scrollX + e.deltaX);
      }
    },
    [zoom, scrollX, setZoom, setScrollX],
  );

  if (!project) return null;

  return (
    <div
      ref={containerRef}
      className="flex flex-col h-full select-none"
      onWheel={handleWheel}
    >
      <TimelineRuler />
      <div className="flex-1 overflow-auto relative">
        <Playhead />
        <TrackList />
      </div>
    </div>
  );
}
