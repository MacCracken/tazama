import { useCallback } from "react";
import { useProjectStore } from "../../../stores/projectStore";
import { usePlaybackStore } from "../../../stores/playbackStore";

const SNAP_THRESHOLD_FRAMES = 5;

/**
 * Returns a function that snaps a frame position to nearby edges.
 * Snap targets: clip start/end edges, playhead position, markers.
 */
export function useSnap() {
  const project = useProjectStore((s) => s.project);
  const playheadFrame = usePlaybackStore((s) => s.position);

  const snap = useCallback(
    (frame: number, excludeClipId?: string): number => {
      if (!project) return frame;

      const targets: number[] = [0, playheadFrame];

      // Clip edges
      for (const track of project.timeline.tracks) {
        for (const clip of track.clips) {
          if (clip.id === excludeClipId) continue;
          targets.push(clip.timeline_start);
          targets.push(clip.timeline_start + clip.duration);
        }
      }

      // Markers
      for (const marker of project.timeline.markers) {
        targets.push(marker.frame);
      }

      // Find closest target within threshold
      let best = frame;
      let bestDist = SNAP_THRESHOLD_FRAMES + 1;
      for (const t of targets) {
        const dist = Math.abs(frame - t);
        if (dist < bestDist) {
          bestDist = dist;
          best = t;
        }
      }

      return best;
    },
    [project, playheadFrame],
  );

  return snap;
}
