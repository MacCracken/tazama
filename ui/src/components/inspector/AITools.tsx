import { useState } from "react";
import type { Clip, Effect } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import * as commands from "../../ipc/commands";

interface AIToolsProps {
  clip: Clip;
  trackId: string;
}

export function AITools({ clip, trackId }: AIToolsProps) {
  const addEffect = useProjectStore((s) => s.addEffect);
  const showToast = useUIStore((s) => s.showToast);
  const [loading, setLoading] = useState<string | null>(null);
  const [highlights, setHighlights] = useState<commands.Highlight[] | null>(null);
  const [subtitles, setSubtitles] = useState<commands.SubtitleCue[] | null>(null);
  const [transitions, setTransitions] = useState<[number, commands.TransitionSuggestion][] | null>(null);
  const [description, setDescription] = useState<commands.ClipDescription | null>(null);

  const mediaPath = clip.media?.path;
  if (!mediaPath) return null;

  const handleAutoColor = async () => {
    setLoading("color");
    try {
      // Analyze the first frame
      const correction = await commands.autoColorCorrect(mediaPath, 0);
      const effect: Effect = {
        id: crypto.randomUUID(),
        kind: {
          ColorGrade: {
            brightness: correction.brightness_offset,
            contrast: correction.contrast_factor,
            saturation: correction.saturation_factor,
            temperature: 0,
          },
        },
        enabled: true,
        keyframe_tracks: [],
      };
      addEffect(trackId, clip.id, effect);
      showToast("Auto color correction applied", "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleDetectHighlights = async () => {
    setLoading("highlights");
    try {
      const results = await commands.detectHighlights(mediaPath, 5);
      setHighlights(results);
      if (results.length === 0) {
        showToast("No highlights detected", "info");
      }
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleTranscribe = async () => {
    setLoading("transcribe");
    try {
      const cues = await commands.transcribeAudio(mediaPath);
      setSubtitles(cues);
      if (cues.length === 0) {
        showToast("No speech detected", "info");
      } else {
        showToast(`${cues.length} subtitle cues generated`, "success");
      }
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleSuggestTransitions = async () => {
    setLoading("transitions");
    try {
      const results = await commands.suggestTransitions(mediaPath, 30);
      setTransitions(results);
      if (results.length === 0) {
        showToast("No scene boundaries found", "info");
      }
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleDescribe = async () => {
    setLoading("describe");
    try {
      const result = await commands.describeClip(mediaPath);
      setDescription(result);
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleRefineSubtitles = async () => {
    if (!subtitles || subtitles.length === 0) return;
    setLoading("refine");
    try {
      const refined = await commands.refineSubtitles(subtitles);
      setSubtitles(refined);
      showToast("Subtitles refined", "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleTranslateSubtitles = async () => {
    if (!subtitles || subtitles.length === 0) return;
    setLoading("translate");
    try {
      const translated = await commands.translateSubtitles(subtitles, "Spanish");
      setSubtitles(translated);
      showToast("Subtitles translated", "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const formatTime = (ms: number) => {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  return (
    <div>
      <label className="text-[10px]" style={{ color: "var(--text-muted)" }}>
        AI Tools
      </label>
      <div className="flex flex-wrap gap-1 mt-1">
        <AIButton
          label="Auto Color"
          loading={loading === "color"}
          disabled={loading !== null}
          onClick={handleAutoColor}
        />
        <AIButton
          label="Highlights"
          loading={loading === "highlights"}
          disabled={loading !== null}
          onClick={handleDetectHighlights}
        />
        <AIButton
          label="Transcribe"
          loading={loading === "transcribe"}
          disabled={loading !== null}
          onClick={handleTranscribe}
        />
        <AIButton
          label="Transitions"
          loading={loading === "transitions"}
          disabled={loading !== null}
          onClick={handleSuggestTransitions}
        />
        <AIButton
          label="Describe"
          loading={loading === "describe"}
          disabled={loading !== null}
          onClick={handleDescribe}
        />
      </div>

      {/* Highlights results */}
      {highlights && highlights.length > 0 && (
        <div className="mt-1.5">
          <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>
            Top highlights:
          </span>
          {highlights.map((h, i) => (
            <div key={i} className="text-[9px] ml-1" style={{ color: "var(--text-secondary)" }}>
              {formatTime(h.start_ms)}–{formatTime(h.end_ms)}{" "}
              <span style={{ color: "var(--text-muted)" }}>score: {h.score.toFixed(2)}</span>
            </div>
          ))}
        </div>
      )}

      {/* Description result */}
      {description && (
        <div className="mt-1.5">
          <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>Description:</span>
          <div className="text-[9px] ml-1" style={{ color: "var(--text-secondary)" }}>
            {description.summary}
          </div>
          {description.tags.length > 0 && (
            <div className="flex flex-wrap gap-0.5 ml-1 mt-0.5">
              {description.tags.map((tag, i) => (
                <span
                  key={i}
                  className="text-[8px] px-1 rounded"
                  style={{ background: "var(--bg-hover)", color: "var(--text-muted)" }}
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Subtitle results */}
      {subtitles && subtitles.length > 0 && (
        <div className="mt-1.5 max-h-[120px] overflow-y-auto">
          <div className="flex items-center gap-1 mb-0.5">
            <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>
              Subtitles ({subtitles.length} cues):
            </span>
            <button
              onClick={handleRefineSubtitles}
              disabled={loading !== null}
              className="text-[8px] px-1 rounded"
              style={{ background: "var(--bg-hover)", color: "var(--text-accent)", opacity: loading ? 0.5 : 1 }}
            >
              {loading === "refine" ? "..." : "Refine"}
            </button>
            <button
              onClick={handleTranslateSubtitles}
              disabled={loading !== null}
              className="text-[8px] px-1 rounded"
              style={{ background: "var(--bg-hover)", color: "var(--text-accent)", opacity: loading ? 0.5 : 1 }}
            >
              {loading === "translate" ? "..." : "Translate"}
            </button>
          </div>
          {subtitles.map((cue) => (
            <div key={cue.index} className="text-[9px] ml-1" style={{ color: "var(--text-secondary)" }}>
              <span style={{ color: "var(--text-muted)" }}>{formatTime(cue.start_ms)}</span>{" "}
              {cue.text}
            </div>
          ))}
        </div>
      )}

      {/* Transition suggestions */}
      {transitions && transitions.length > 0 && (
        <div className="mt-1.5">
          <span className="text-[9px]" style={{ color: "var(--text-muted)" }}>
            Suggested transitions:
          </span>
          {transitions.map(([ms, sug], i) => (
            <div key={i} className="text-[9px] ml-1" style={{ color: "var(--text-secondary)" }}>
              {formatTime(ms)}: {sug.kind}
              {sug.duration_frames > 0 && ` (${sug.duration_frames}f)`}
              <div className="ml-1 text-[8px]" style={{ color: "var(--text-muted)" }}>
                {sug.reason}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function AIButton({
  label,
  loading,
  disabled,
  onClick,
}: {
  label: string;
  loading: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="text-[10px] px-1.5 py-0.5 rounded"
      style={{
        background: "var(--bg-hover)",
        color: disabled ? "var(--text-muted)" : "var(--text-accent)",
        opacity: disabled && !loading ? 0.5 : 1,
      }}
    >
      {loading ? "..." : label}
    </button>
  );
}
