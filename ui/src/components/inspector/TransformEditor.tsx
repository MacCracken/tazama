import { useState } from "react";
import type { Effect } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

interface TransformEditorProps {
  trackId: string;
  clipId: string;
  effect?: Effect;
}

export function TransformEditor({ trackId, clipId, effect }: TransformEditorProps) {
  const addEffect = useProjectStore((s) => s.addEffect);

  const existing =
    effect && "Transform" in effect.kind ? effect.kind.Transform : null;

  const [scaleX, setScaleX] = useState(existing?.scale_x ?? 1.0);
  const [scaleY, setScaleY] = useState(existing?.scale_y ?? 1.0);
  const [translateX, setTranslateX] = useState(existing?.translate_x ?? 0);
  const [translateY, setTranslateY] = useState(existing?.translate_y ?? 0);

  const handleApply = () => {
    const transformEffect: Effect = {
      id: effect?.id ?? crypto.randomUUID(),
      kind: {
        Transform: {
          scale_x: scaleX,
          scale_y: scaleY,
          translate_x: translateX,
          translate_y: translateY,
        },
      },
      enabled: true,
      keyframe_tracks: effect?.keyframe_tracks ?? [],
    };
    addEffect(trackId, clipId, transformEffect);
  };

  return (
    <div className="transform-editor">
      <div className="transform-field">
        <label>Scale X</label>
        <input
          type="range"
          min={0.1}
          max={3}
          step={0.01}
          value={scaleX}
          onChange={(e) => setScaleX(parseFloat(e.target.value))}
        />
        <span>{scaleX.toFixed(2)}</span>
      </div>
      <div className="transform-field">
        <label>Scale Y</label>
        <input
          type="range"
          min={0.1}
          max={3}
          step={0.01}
          value={scaleY}
          onChange={(e) => setScaleY(parseFloat(e.target.value))}
        />
        <span>{scaleY.toFixed(2)}</span>
      </div>
      <div className="transform-field">
        <label>Translate X</label>
        <input
          type="range"
          min={-1}
          max={1}
          step={0.01}
          value={translateX}
          onChange={(e) => setTranslateX(parseFloat(e.target.value))}
        />
        <span>{translateX.toFixed(2)}</span>
      </div>
      <div className="transform-field">
        <label>Translate Y</label>
        <input
          type="range"
          min={-1}
          max={1}
          step={0.01}
          value={translateY}
          onChange={(e) => setTranslateY(parseFloat(e.target.value))}
        />
        <span>{translateY.toFixed(2)}</span>
      </div>
      <button onClick={handleApply}>Apply Transform</button>
    </div>
  );
}
