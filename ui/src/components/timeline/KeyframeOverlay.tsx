import type { Effect } from "../../types";

interface KeyframeOverlayProps {
  effects: Effect[];
  clipStart: number;
  clipDuration: number;
  pixelsPerFrame: number;
}

export function KeyframeOverlay({
  effects,
  clipStart,
  clipDuration,
  pixelsPerFrame,
}: KeyframeOverlayProps) {
  const allKeyframes: { frame: number; param: string }[] = [];

  for (const effect of effects) {
    for (const track of effect.keyframe_tracks) {
      for (const kf of track.keyframes) {
        if (kf.frame >= clipStart && kf.frame < clipStart + clipDuration) {
          allKeyframes.push({
            frame: kf.frame,
            param: track.parameter,
          });
        }
      }
    }
  }

  if (allKeyframes.length === 0) return null;

  return (
    <div className="keyframe-overlay">
      {allKeyframes.map((kf, i) => {
        const x = (kf.frame - clipStart) * pixelsPerFrame;
        return (
          <div
            key={`${kf.param}-${kf.frame}-${i}`}
            className="keyframe-diamond"
            style={{
              left: `${x}px`,
            }}
            title={`${kf.param} @ frame ${kf.frame}`}
          />
        );
      })}
    </div>
  );
}
