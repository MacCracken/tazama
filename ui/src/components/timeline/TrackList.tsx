import { useProjectStore } from "../../stores/projectStore";
import { TrackRow } from "./TrackRow";

export function TrackList() {
  const tracks = useProjectStore((s) => s.project?.timeline.tracks ?? []);

  return (
    <div className="flex flex-col">
      {tracks.map((track) => (
        <TrackRow key={track.id} track={track} />
      ))}
      {tracks.length === 0 && (
        <div
          className="flex items-center justify-center h-24"
          style={{ color: "var(--text-muted)" }}
        >
          No tracks — add a track to get started
        </div>
      )}
    </div>
  );
}
