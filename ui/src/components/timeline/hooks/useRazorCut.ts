import { useCallback } from "react";
import type { Clip } from "../../../types";
import { useProjectStore } from "../../../stores/projectStore";

export function useRazorCut(
  trackId: string,
  clip: Clip,
  zoom: number,
  _scrollX: number,
) {
  const splitClip = useProjectStore((s) => s.splitClip);

  const onClick = useCallback(
    (e: React.MouseEvent) => {
      const rect = e.currentTarget.getBoundingClientRect();
      const localX = e.clientX - rect.left;
      const frame = clip.timeline_start + Math.round(localX / zoom);

      if (
        frame > clip.timeline_start &&
        frame < clip.timeline_start + clip.duration
      ) {
        splitClip(trackId, clip.id, frame);
      }
    },
    [clip, trackId, zoom, splitClip],
  );

  return { onClick };
}
