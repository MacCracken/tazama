import { useState, useRef, useEffect } from "react";
import type { Clip, Effect, EffectKind } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { EffectEditor } from "./EffectEditor";
import { KeyframeEditor } from "./KeyframeEditor";

const EFFECT_PRESETS: { label: string; kind: EffectKind }[] = [
  { label: "Color Grade", kind: { ColorGrade: { brightness: 0, contrast: 1, saturation: 1, temperature: 0 } } },
  { label: "Volume", kind: { Volume: { gain_db: 0 } } },
  { label: "Fade In", kind: { FadeIn: { duration_frames: 15 } } },
  { label: "Fade Out", kind: { FadeOut: { duration_frames: 15 } } },
  { label: "Speed", kind: { Speed: { factor: 1 } } },
  { label: "EQ", kind: { Eq: { low_gain_db: 0, mid_gain_db: 0, high_gain_db: 0 } } },
  { label: "Compressor", kind: { Compressor: { threshold_db: -20, ratio: 4, attack_ms: 10, release_ms: 100 } } },
  { label: "Noise Reduction", kind: { NoiseReduction: { strength: 0.5 } } },
  { label: "Reverb", kind: { Reverb: { room_size: 0.5, damping: 0.5, wet: 0.3 } } },
  { label: "Loudness Normalize", kind: { LoudnessNormalize: { target_lufs: -14 } } },
  { label: "Crop", kind: { Crop: { left: 0, top: 0, right: 0, bottom: 0 } } },
  { label: "Transform", kind: { Transform: { scale_x: 1, scale_y: 1, translate_x: 0, translate_y: 0 } } },
];

interface EffectListProps {
  clip: Clip;
  trackId: string;
}

export function EffectList({ clip, trackId }: EffectListProps) {
  const addEffect = useProjectStore((s) => s.addEffect);
  const removeEffect = useProjectStore((s) => s.removeEffect);
  const [showMenu, setShowMenu] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!showMenu) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowMenu(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showMenu]);

  const handleAdd = (kind: EffectKind) => {
    const effect: Effect = {
      id: crypto.randomUUID(),
      kind,
      enabled: true,
      keyframe_tracks: [],
    };
    addEffect(trackId, clip.id, effect);
    setShowMenu(false);
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-1">
        <label className="text-[10px]" style={{ color: "var(--text-muted)" }}>
          Effects ({clip.effects.length})
        </label>
        <div className="relative" ref={menuRef}>
          <button
            onClick={() => setShowMenu(!showMenu)}
            className="text-[10px] px-1 rounded hover:bg-[var(--bg-hover)]"
            style={{ color: "var(--text-accent)" }}
          >
            + Add
          </button>
          {showMenu && (
            <div
              className="absolute right-0 top-full mt-1 z-50 rounded shadow-lg py-1 min-w-[140px] max-h-[240px] overflow-y-auto"
              style={{
                background: "var(--bg-secondary)",
                border: "1px solid var(--border-default)",
              }}
            >
              {EFFECT_PRESETS.map((preset) => (
                <button
                  key={preset.label}
                  onClick={() => handleAdd(preset.kind)}
                  className="block w-full text-left px-2 py-1 text-[10px] hover:bg-[var(--bg-hover)]"
                  style={{ color: "var(--text-primary)" }}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          )}
        </div>
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
          <KeyframeEditor effect={effect} trackId={trackId} clipId={clip.id} />
        </div>
      ))}
    </div>
  );
}
