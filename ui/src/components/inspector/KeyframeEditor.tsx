import { useState } from "react";
import type { Effect, KeyframeTrack, Keyframe, Interpolation } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

interface KeyframeEditorProps {
  trackId: string;
  clipId: string;
  effect: Effect;
}

export function KeyframeEditor({ trackId, clipId, effect }: KeyframeEditorProps) {
  const setKeyframeTracks = useProjectStore((s) => s.setKeyframeTracks);
  const [selectedParam, setSelectedParam] = useState<string | null>(null);

  const paramNames = getAnimatableParams(effect);
  const tracks = effect.keyframe_tracks;

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
          ? {
              ...t,
              keyframes: t.keyframes.filter((k) => k.id !== keyframeId),
            }
          : t,
      )
      .filter((t) => t.keyframes.length > 0);

    setKeyframeTracks(trackId, clipId, effect.id, updatedTracks);
  };

  const handleToggleAnimate = (param: string) => {
    const hasTrack = tracks.some((t) => t.parameter === param);
    if (hasTrack) {
      // Remove all keyframes for this parameter
      const updatedTracks = tracks.filter((t) => t.parameter !== param);
      setKeyframeTracks(trackId, clipId, effect.id, updatedTracks);
    } else {
      // Create initial keyframe track with a single keyframe at frame 0
      const defaultValue = getDefaultValue(effect, param);
      handleAddKeyframe(param, 0, defaultValue);
    }
  };

  return (
    <div className="keyframe-editor">
      <div className="keyframe-params">
        {paramNames.map((param) => {
          const track = tracks.find((t) => t.parameter === param);
          const isAnimated = !!track;

          return (
            <div key={param} className="keyframe-param">
              <span className="param-name">{param}</span>
              <button
                className={`animate-toggle ${isAnimated ? "active" : ""}`}
                onClick={() => handleToggleAnimate(param)}
                title={isAnimated ? "Remove animation" : "Animate"}
              >
                {isAnimated ? "K" : "+K"}
              </button>

              {isAnimated && track && (
                <div
                  className={`keyframe-list ${selectedParam === param ? "expanded" : ""}`}
                  onClick={() =>
                    setSelectedParam(selectedParam === param ? null : param)
                  }
                >
                  <span>{track.keyframes.length} keyframes</span>
                  {selectedParam === param &&
                    track.keyframes.map((kf) => (
                      <div key={kf.id} className="keyframe-item">
                        <span>
                          F{kf.frame}: {kf.value.toFixed(2)}
                        </span>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            handleRemoveKeyframe(param, kf.id);
                          }}
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
    </div>
  );
}

function getAnimatableParams(effect: Effect): string[] {
  const kind = effect.kind;
  if ("ColorGrade" in kind) return ["brightness", "contrast", "saturation", "temperature"];
  if ("Crop" in kind) return ["left", "top", "right", "bottom"];
  if ("Speed" in kind) return ["factor"];
  if ("Volume" in kind) return ["gain_db"];
  if ("Transform" in kind) return ["scale_x", "scale_y", "translate_x", "translate_y"];
  if ("Text" in kind) return ["x", "y", "font_size"];
  if ("Eq" in kind) return ["low_gain_db", "mid_gain_db", "high_gain_db"];
  if ("Compressor" in kind) return ["threshold_db", "ratio", "attack_ms", "release_ms"];
  if ("Reverb" in kind) return ["room_size", "damping", "wet"];
  if ("NoiseReduction" in kind) return ["strength"];
  return [];
}

function getDefaultValue(effect: Effect, param: string): number {
  const kind = effect.kind;
  if ("ColorGrade" in kind) {
    const v = kind.ColorGrade;
    return (v as Record<string, number>)[param] ?? 0;
  }
  if ("Crop" in kind) {
    return (kind.Crop as Record<string, number>)[param] ?? 0;
  }
  if ("Speed" in kind) return kind.Speed.factor;
  if ("Volume" in kind) return kind.Volume.gain_db;
  if ("Transform" in kind) {
    return (kind.Transform as Record<string, number>)[param] ?? 0;
  }
  return 0;
}
