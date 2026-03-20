import { useState } from "react";
import type { Effect, KeyframeTrack, Keyframe, Interpolation } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { usePlaybackStore } from "../../stores/playbackStore";

interface KeyframeEditorProps {
  trackId: string;
  clipId: string;
  effect: Effect;
}

export function KeyframeEditor({ trackId, clipId, effect }: KeyframeEditorProps) {
  const setKeyframeTracks = useProjectStore((s) => s.setKeyframeTracks);
  const position = usePlaybackStore((s) => s.position);
  const [expandedParam, setExpandedParam] = useState<string | null>(null);

  const paramNames = getAnimatableParams(effect);
  const tracks = effect.keyframe_tracks;

  if (paramNames.length === 0) return null;

  const handleAddKeyframe = (param: string, frame: number, value: number) => {
    const existingTrack = tracks.find((t) => t.parameter === param);
    const newKeyframe: Keyframe = {
      id: crypto.randomUUID(),
      frame,
      value,
      interpolation: "Linear" as Interpolation,
    };

    let updatedTracks: KeyframeTrack[];
    if (existingTrack) {
      updatedTracks = tracks.map((t) =>
        t.parameter === param
          ? {
              ...t,
              keyframes: [...t.keyframes, newKeyframe].sort(
                (a, b) => a.frame - b.frame,
              ),
            }
          : t,
      );
    } else {
      updatedTracks = [
        ...tracks,
        {
          id: crypto.randomUUID(),
          parameter: param,
          keyframes: [newKeyframe],
        },
      ];
    }

    setKeyframeTracks(trackId, clipId, effect.id, updatedTracks);
  };

  const handleRemoveKeyframe = (param: string, keyframeId: string) => {
    const updatedTracks = tracks
      .map((t) =>
        t.parameter === param
          ? { ...t, keyframes: t.keyframes.filter((k) => k.id !== keyframeId) }
          : t,
      )
      .filter((t) => t.keyframes.length > 0);

    setKeyframeTracks(trackId, clipId, effect.id, updatedTracks);
  };

  const handleToggleAnimate = (param: string) => {
    const hasTrack = tracks.some((t) => t.parameter === param);
    if (hasTrack) {
      const updatedTracks = tracks.filter((t) => t.parameter !== param);
      setKeyframeTracks(trackId, clipId, effect.id, updatedTracks);
    } else {
      const defaultValue = getDefaultValue(effect, param);
      handleAddKeyframe(param, 0, defaultValue);
    }
  };

  return (
    <div className="mt-1 space-y-0.5">
      {paramNames.map((param) => {
        const track = tracks.find((t) => t.parameter === param);
        const isAnimated = !!track;
        const isExpanded = expandedParam === param;

        return (
          <div key={param}>
            <div className="flex items-center gap-1">
              <span
                className="text-[9px] flex-1 truncate"
                style={{ color: "var(--text-muted)" }}
              >
                {param}
              </span>
              {isAnimated && (
                <button
                  onClick={() => {
                    const val = getDefaultValue(effect, param);
                    handleAddKeyframe(param, position, val);
                  }}
                  className="text-[9px] px-0.5 rounded hover:bg-[var(--bg-hover)]"
                  style={{ color: "var(--text-accent)" }}
                  title={`Add keyframe at frame ${position}`}
                >
                  +
                </button>
              )}
              <button
                onClick={() => handleToggleAnimate(param)}
                className="text-[9px] px-1 rounded font-bold"
                style={{
                  background: isAnimated ? "var(--accent-primary)" : "var(--bg-hover)",
                  color: isAnimated ? "#fff" : "var(--text-muted)",
                }}
                title={isAnimated ? "Remove animation" : "Animate"}
              >
                K
              </button>
            </div>
            {isAnimated && track && (
              <div
                className="ml-2 cursor-pointer"
                onClick={() => setExpandedParam(isExpanded ? null : param)}
              >
                <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>
                  {track.keyframes.length} keyframe{track.keyframes.length !== 1 ? "s" : ""}
                </span>
                {isExpanded &&
                  track.keyframes.map((kf) => (
                    <div
                      key={kf.id}
                      className="flex items-center justify-between text-[9px] ml-1"
                    >
                      <span style={{ color: "var(--text-secondary)" }}>
                        F{kf.frame}: {kf.value.toFixed(2)}
                      </span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleRemoveKeyframe(param, kf.id);
                        }}
                        className="px-0.5 rounded hover:bg-[var(--bg-hover)]"
                        style={{ color: "var(--error)" }}
                      >
                        x
                      </button>
                    </div>
                  ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function getAnimatableParams(effect: Effect): string[] {
  const kind = effect.kind;
  if ("ColorGrade" in kind) return ["brightness", "contrast", "saturation", "temperature"];
  if ("Crop" in kind) return ["left", "top", "right", "bottom"];
  if ("Speed" in kind) return ["factor"];
  if ("Volume" in kind) return ["gain_db"];
  if ("FadeIn" in kind) return ["duration_frames"];
  if ("FadeOut" in kind) return ["duration_frames"];
  if ("Transform" in kind) return ["scale_x", "scale_y", "translate_x", "translate_y"];
  if ("Text" in kind) return ["x", "y", "font_size"];
  if ("Eq" in kind) return ["low_gain_db", "mid_gain_db", "high_gain_db"];
  if ("Compressor" in kind) return ["threshold_db", "ratio", "attack_ms", "release_ms"];
  if ("Reverb" in kind) return ["room_size", "damping", "wet"];
  if ("NoiseReduction" in kind) return ["strength"];
  if ("LoudnessNormalize" in kind) return ["target_lufs"];
  return [];
}

function getDefaultValue(effect: Effect, param: string): number {
  const kind = effect.kind;
  if ("ColorGrade" in kind) return (kind.ColorGrade as Record<string, number>)[param] ?? 0;
  if ("Crop" in kind) return (kind.Crop as Record<string, number>)[param] ?? 0;
  if ("Speed" in kind) return kind.Speed.factor;
  if ("Volume" in kind) return kind.Volume.gain_db;
  if ("FadeIn" in kind) return kind.FadeIn.duration_frames;
  if ("FadeOut" in kind) return kind.FadeOut.duration_frames;
  if ("Transform" in kind) return (kind.Transform as Record<string, number>)[param] ?? 0;
  if ("Text" in kind) {
    const t = kind.Text;
    if (param === "x") return t.x;
    if (param === "y") return t.y;
    if (param === "font_size") return t.font_size;
    return 0;
  }
  if ("Eq" in kind) return (kind.Eq as Record<string, number>)[param] ?? 0;
  if ("Compressor" in kind) return (kind.Compressor as Record<string, number>)[param] ?? 0;
  if ("Reverb" in kind) return (kind.Reverb as Record<string, number>)[param] ?? 0;
  if ("NoiseReduction" in kind) return kind.NoiseReduction.strength;
  if ("LoudnessNormalize" in kind) return kind.LoudnessNormalize.target_lufs;
  return 0;
}
