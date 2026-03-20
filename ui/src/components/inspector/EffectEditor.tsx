import type { Effect } from "../../types";

interface EffectEditorProps {
  effect: Effect;
  trackId: string;
  clipId: string;
}

export function EffectEditor({ effect }: EffectEditorProps) {
  const kind = effect.kind;

  if ("ColorGrade" in kind) {
    const { brightness, contrast, saturation, temperature } = kind.ColorGrade;
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div>Brightness: {brightness.toFixed(2)}</div>
        <div>Contrast: {contrast.toFixed(2)}</div>
        <div>Saturation: {saturation.toFixed(2)}</div>
        <div>Temperature: {temperature.toFixed(2)}</div>
      </div>
    );
  }

  if ("Crop" in kind) {
    const { left, top, right, bottom } = kind.Crop;
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div>L: {left.toFixed(2)} T: {top.toFixed(2)}</div>
        <div>R: {right.toFixed(2)} B: {bottom.toFixed(2)}</div>
      </div>
    );
  }

  if ("Speed" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Factor: {kind.Speed.factor.toFixed(2)}x
      </div>
    );
  }

  if ("Volume" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Gain: {kind.Volume.gain_db.toFixed(1)} dB
      </div>
    );
  }

  if ("FadeIn" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Duration: {kind.FadeIn.duration_frames} frames
      </div>
    );
  }

  if ("FadeOut" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Duration: {kind.FadeOut.duration_frames} frames
      </div>
    );
  }

  if ("Transition" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        {kind.Transition.kind} — {kind.Transition.duration_frames} frames
      </div>
    );
  }

  if ("Eq" in kind) {
    const { low_gain_db, mid_gain_db, high_gain_db } = kind.Eq;
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div>Low: {low_gain_db.toFixed(1)} dB</div>
        <div>Mid: {mid_gain_db.toFixed(1)} dB</div>
        <div>High: {high_gain_db.toFixed(1)} dB</div>
      </div>
    );
  }

  if ("Compressor" in kind) {
    const { threshold_db, ratio, attack_ms, release_ms } = kind.Compressor;
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div>Threshold: {threshold_db.toFixed(1)} dB</div>
        <div>Ratio: {ratio.toFixed(1)}:1</div>
        <div>Attack: {attack_ms.toFixed(0)} ms</div>
        <div>Release: {release_ms.toFixed(0)} ms</div>
      </div>
    );
  }

  if ("NoiseReduction" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Strength: {(kind.NoiseReduction.strength * 100).toFixed(0)}%
      </div>
    );
  }

  if ("Reverb" in kind) {
    const { room_size, damping, wet } = kind.Reverb;
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div>Room: {(room_size * 100).toFixed(0)}%</div>
        <div>Damping: {(damping * 100).toFixed(0)}%</div>
        <div>Wet: {(wet * 100).toFixed(0)}%</div>
      </div>
    );
  }

  if ("LoudnessNormalize" in kind) {
    return (
      <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
        Target: {kind.LoudnessNormalize.target_lufs.toFixed(1)} LUFS
      </div>
    );
  }

  if ("Transform" in kind) {
    const { scale_x, scale_y, translate_x, translate_y } = kind.Transform;
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div>Scale: {scale_x.toFixed(2)} x {scale_y.toFixed(2)}</div>
        <div>Position: {translate_x.toFixed(0)}, {translate_y.toFixed(0)}</div>
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
    return (
      <div className="space-y-1 text-[10px]" style={{ color: "var(--text-secondary)" }}>
        <div className="truncate">"{kind.Text.content}"</div>
        <div>{kind.Text.font_family} {kind.Text.font_size}pt</div>
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
