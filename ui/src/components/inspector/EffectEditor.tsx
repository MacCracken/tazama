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

  return null;
}
