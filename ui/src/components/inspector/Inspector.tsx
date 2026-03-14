import { useUIStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";
import { ClipInspector } from "./ClipInspector";
import { TrackInspector } from "./TrackInspector";
import { NoSelection } from "./NoSelection";

export function Inspector() {
  const selectedClipId = useUIStore((s) => s.selectedClipId);
  const selectedTrackId = useUIStore((s) => s.selectedTrackId);
  const project = useProjectStore((s) => s.project);

  if (!project) return <NoSelection />;

  if (selectedClipId) {
    for (const track of project.timeline.tracks) {
      const clip = track.clips.find((c) => c.id === selectedClipId);
      if (clip) {
        return <ClipInspector clip={clip} trackId={track.id} />;
      }
    }
  }

  if (selectedTrackId) {
    const track = project.timeline.tracks.find((t) => t.id === selectedTrackId);
    if (track) {
      return <TrackInspector track={track} />;
    }
  }

  return <NoSelection />;
}
