import type { Clip, Effect } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { EffectEditor } from "./EffectEditor";

interface EffectListProps {
  clip: Clip;
  trackId: string;
}

export function EffectList({ clip, trackId }: EffectListProps) {
  const addEffect = useProjectStore((s) => s.addEffect);
  const removeEffect = useProjectStore((s) => s.removeEffect);

  const handleAddEffect = () => {
    const effect: Effect = {
      id: crypto.randomUUID(),
      kind: {
        ColorGrade: {
          brightness: 0,
          contrast: 1,
          saturation: 1,
          temperature: 0,
        },
      },
      enabled: true,
      keyframe_tracks: [],
    };
    addEffect(trackId, clip.id, effect);
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-1">
        <label className="text-[10px]" style={{ color: "var(--text-muted)" }}>
          Effects ({clip.effects.length})
        </label>
        <button
          onClick={handleAddEffect}
          className="text-[10px] px-1 rounded hover:bg-[var(--bg-hover)]"
          style={{ color: "var(--text-accent)" }}
        >
          + Add
        </button>
      </div>
      {clip.effects.map((effect) => (
        <div key={effect.id} className="mb-2">
          <div className="flex items-center justify-between mb-0.5">
            <span className="text-[10px] font-medium" style={{ color: "var(--text-primary)" }}>
              {Object.keys(effect.kind)[0]}
            </span>
            <button
              onClick={() => removeEffect(trackId, clip.id, effect.id)}
              className="text-[10px] px-1 rounded hover:bg-[var(--bg-hover)]"
              style={{ color: "var(--error)" }}
            >
              x
            </button>
          </div>
          <EffectEditor effect={effect} trackId={trackId} clipId={clip.id} />
        </div>
      ))}
    </div>
  );
}
