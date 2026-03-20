import { useCallback } from "react";
import type { Effect, EffectKind } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

interface EffectEditorProps {
  effect: Effect;
  trackId: string;
  clipId: string;
}

// Reusable slider row
function Param({
  label,
  value,
  min,
  max,
  step,
  suffix,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  suffix?: string;
  onChange: (v: number) => void;
}) {
  return (
    <div className="flex items-center gap-1">
      <span
        className="text-[10px] w-[60px] flex-shrink-0 truncate"
        style={{ color: "var(--text-muted)" }}
      >
        {label}
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="flex-1 h-3"
      />
      <span
        className="text-[10px] w-[42px] text-right font-mono flex-shrink-0"
        style={{ color: "var(--text-secondary)" }}
      >
        {value.toFixed(step < 1 ? (step < 0.1 ? 2 : 1) : 0)}
        {suffix ?? ""}
      </span>
    </div>
  );
}

export function EffectEditor({ effect, trackId, clipId }: EffectEditorProps) {
  const updateEffect = useProjectStore((s) => s.updateEffect);

  const update = useCallback(
    (kind: EffectKind) => updateEffect(trackId, clipId, effect.id, kind),
    [updateEffect, trackId, clipId, effect.id],
  );

  const kind = effect.kind;

  if ("ColorGrade" in kind) {
    const k = kind.ColorGrade;
    const set = (field: string, v: number) =>
      update({ ColorGrade: { ...k, [field]: v } });
    return (
      <div className="space-y-1">
        <Param label="Brightness" value={k.brightness} min={-1} max={1} step={0.01} onChange={(v) => set("brightness", v)} />
        <Param label="Contrast" value={k.contrast} min={0} max={3} step={0.01} onChange={(v) => set("contrast", v)} />
        <Param label="Saturation" value={k.saturation} min={0} max={3} step={0.01} onChange={(v) => set("saturation", v)} />
        <Param label="Temperature" value={k.temperature} min={-1} max={1} step={0.01} onChange={(v) => set("temperature", v)} />
      </div>
    );
  }

  if ("Crop" in kind) {
    const k = kind.Crop;
    const set = (field: string, v: number) =>
      update({ Crop: { ...k, [field]: v } });
    return (
      <div className="space-y-1">
        <Param label="Left" value={k.left} min={0} max={1} step={0.01} onChange={(v) => set("left", v)} />
        <Param label="Top" value={k.top} min={0} max={1} step={0.01} onChange={(v) => set("top", v)} />
        <Param label="Right" value={k.right} min={0} max={1} step={0.01} onChange={(v) => set("right", v)} />
        <Param label="Bottom" value={k.bottom} min={0} max={1} step={0.01} onChange={(v) => set("bottom", v)} />
      </div>
    );
  }

  if ("Speed" in kind) {
    return (
      <Param
        label="Factor"
        value={kind.Speed.factor}
        min={0.1}
        max={8}
        step={0.1}
        suffix="x"
        onChange={(v) => update({ Speed: { factor: v } })}
      />
    );
  }

  if ("Volume" in kind) {
    return (
      <Param
        label="Gain"
        value={kind.Volume.gain_db}
        min={-60}
        max={24}
        step={0.5}
        suffix="dB"
        onChange={(v) => update({ Volume: { gain_db: v } })}
      />
    );
  }

  if ("FadeIn" in kind) {
    return (
      <Param
        label="Duration"
        value={kind.FadeIn.duration_frames}
        min={1}
        max={300}
        step={1}
        suffix="f"
        onChange={(v) => update({ FadeIn: { duration_frames: Math.round(v) } })}
      />
    );
  }

  if ("FadeOut" in kind) {
    return (
      <Param
        label="Duration"
        value={kind.FadeOut.duration_frames}
        min={1}
        max={300}
        step={1}
        suffix="f"
        onChange={(v) => update({ FadeOut: { duration_frames: Math.round(v) } })}
      />
    );
  }

  if ("Transition" in kind) {
    return (
      <div className="space-y-1">
        <div className="flex items-center gap-1">
          <span className="text-[10px] w-[60px]" style={{ color: "var(--text-muted)" }}>Kind</span>
          <select
            value={kind.Transition.kind}
            onChange={(e) =>
              update({
                Transition: {
                  ...kind.Transition,
                  kind: e.target.value as "Cut" | "Dissolve" | "Wipe" | "Fade",
                },
              })
            }
            className="flex-1 text-[10px] px-1 py-0.5 rounded"
            style={{
              background: "var(--bg-primary)",
              border: "1px solid var(--border-default)",
              color: "var(--text-primary)",
            }}
          >
            {["Cut", "Dissolve", "Wipe", "Fade"].map((t) => (
              <option key={t} value={t}>{t}</option>
            ))}
          </select>
        </div>
        <Param
          label="Duration"
          value={kind.Transition.duration_frames}
          min={1}
          max={120}
          step={1}
          suffix="f"
          onChange={(v) =>
            update({ Transition: { ...kind.Transition, duration_frames: Math.round(v) } })
          }
        />
      </div>
    );
  }

  if ("Eq" in kind) {
    const k = kind.Eq;
    const set = (field: string, v: number) =>
      update({ Eq: { ...k, [field]: v } });
    return (
      <div className="space-y-1">
        <Param label="Low" value={k.low_gain_db} min={-24} max={24} step={0.5} suffix="dB" onChange={(v) => set("low_gain_db", v)} />
        <Param label="Mid" value={k.mid_gain_db} min={-24} max={24} step={0.5} suffix="dB" onChange={(v) => set("mid_gain_db", v)} />
        <Param label="High" value={k.high_gain_db} min={-24} max={24} step={0.5} suffix="dB" onChange={(v) => set("high_gain_db", v)} />
      </div>
    );
  }

  if ("Compressor" in kind) {
    const k = kind.Compressor;
    const set = (field: string, v: number) =>
      update({ Compressor: { ...k, [field]: v } });
    return (
      <div className="space-y-1">
        <Param label="Threshold" value={k.threshold_db} min={-60} max={0} step={0.5} suffix="dB" onChange={(v) => set("threshold_db", v)} />
        <Param label="Ratio" value={k.ratio} min={1} max={20} step={0.5} onChange={(v) => set("ratio", v)} />
        <Param label="Attack" value={k.attack_ms} min={0.1} max={200} step={1} suffix="ms" onChange={(v) => set("attack_ms", v)} />
        <Param label="Release" value={k.release_ms} min={1} max={2000} step={10} suffix="ms" onChange={(v) => set("release_ms", v)} />
      </div>
    );
  }

  if ("NoiseReduction" in kind) {
    return (
      <Param
        label="Strength"
        value={kind.NoiseReduction.strength}
        min={0}
        max={1}
        step={0.01}
        onChange={(v) => update({ NoiseReduction: { strength: v } })}
      />
    );
  }

  if ("Reverb" in kind) {
    const k = kind.Reverb;
    const set = (field: string, v: number) =>
      update({ Reverb: { ...k, [field]: v } });
    return (
      <div className="space-y-1">
        <Param label="Room" value={k.room_size} min={0} max={1} step={0.01} onChange={(v) => set("room_size", v)} />
        <Param label="Damping" value={k.damping} min={0} max={1} step={0.01} onChange={(v) => set("damping", v)} />
        <Param label="Wet" value={k.wet} min={0} max={1} step={0.01} onChange={(v) => set("wet", v)} />
      </div>
    );
  }

  if ("LoudnessNormalize" in kind) {
    return (
      <Param
        label="Target"
        value={kind.LoudnessNormalize.target_lufs}
        min={-36}
        max={0}
        step={0.5}
        suffix=" LUFS"
        onChange={(v) => update({ LoudnessNormalize: { target_lufs: v } })}
      />
    );
  }

  if ("Transform" in kind) {
    const k = kind.Transform;
    const set = (field: string, v: number) =>
      update({ Transform: { ...k, [field]: v } });
    return (
      <div className="space-y-1">
        <Param label="Scale X" value={k.scale_x} min={0.01} max={5} step={0.01} onChange={(v) => set("scale_x", v)} />
        <Param label="Scale Y" value={k.scale_y} min={0.01} max={5} step={0.01} onChange={(v) => set("scale_y", v)} />
        <Param label="X" value={k.translate_x} min={-2000} max={2000} step={1} onChange={(v) => set("translate_x", v)} />
        <Param label="Y" value={k.translate_y} min={-2000} max={2000} step={1} onChange={(v) => set("translate_y", v)} />
      </div>
    );
  }

  if ("Lut" in kind) {
    return (
      <div className="text-[10px] truncate" style={{ color: "var(--text-secondary)" }}>
        LUT: {kind.Lut.lut_path.split("/").pop() ?? kind.Lut.lut_path}
      </div>
    );
  }

  if ("Text" in kind) {
    const k = kind.Text;
    return (
      <div className="space-y-1">
        <div className="flex items-center gap-1">
          <span className="text-[10px] w-[60px]" style={{ color: "var(--text-muted)" }}>Text</span>
          <input
            type="text"
            value={k.content}
            onChange={(e) => update({ Text: { ...k, content: e.target.value } })}
            className="flex-1 text-[10px] px-1 py-0.5 rounded"
            style={{
              background: "var(--bg-primary)",
              border: "1px solid var(--border-default)",
              color: "var(--text-primary)",
            }}
          />
        </div>
        <Param label="Size" value={k.font_size} min={4} max={200} step={1} suffix="pt" onChange={(v) => update({ Text: { ...k, font_size: v } })} />
        <Param label="X" value={k.x} min={0} max={2000} step={1} onChange={(v) => update({ Text: { ...k, x: v } })} />
        <Param label="Y" value={k.y} min={0} max={2000} step={1} onChange={(v) => update({ Text: { ...k, y: v } })} />
      </div>
    );
  }

  if ("Plugin" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Plugin: {kind.Plugin.plugin_id}
      </div>
    );
  }

  return null;
}
