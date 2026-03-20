import type { Clip } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { EffectList } from "./EffectList";
import { LoudnessMeter } from "./LoudnessMeter";

interface ClipInspectorProps {
  clip: Clip;
  trackId: string;
}

export function ClipInspector({ clip, trackId }: ClipInspectorProps) {
  const setClipOpacity = useProjectStore((s) => s.setClipOpacity);
  const setClipVolume = useProjectStore((s) => s.setClipVolume);
  const renameClip = useProjectStore((s) => s.renameClip);

  return (
    <div className="flex flex-col h-full">
      <div
        className="px-2 py-1.5 border-b"
        style={{
          borderColor: "var(--border-default)",
          background: "var(--bg-tertiary)",
        }}
      >
        <span className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
          Clip Inspector
        </span>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-3">
        <div>
          <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
            Name
          </label>
          <input
            type="text"
            value={clip.name}
            onChange={(e) => renameClip(trackId, clip.id, e.target.value)}
            className="w-full px-1.5 py-1 rounded text-xs"
            style={{
              background: "var(--bg-primary)",
              border: "1px solid var(--border-default)",
            }}
          />
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
              Start
            </label>
            <div className="text-xs font-mono" style={{ color: "var(--text-primary)" }}>
              {clip.timeline_start}
            </div>
          </div>
          <div>
            <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
              Duration
            </label>
            <div className="text-xs font-mono" style={{ color: "var(--text-primary)" }}>
              {clip.duration}
            </div>
          </div>
        </div>
        <div>
          <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
            Kind
          </label>
          <div className="text-xs" style={{ color: "var(--text-primary)" }}>
            {clip.kind}
          </div>
        </div>
        <div>
          <label className="block text-[10px] mb-1" style={{ color: "var(--text-muted)" }}>
            Opacity: {Math.round(clip.opacity * 100)}%
          </label>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={clip.opacity}
            onChange={(e) =>
              setClipOpacity(trackId, clip.id, parseFloat(e.target.value))
            }
            className="w-full"
          />
        </div>
        <div>
          <label className="block text-[10px] mb-1" style={{ color: "var(--text-muted)" }}>
            Volume: {Math.round(clip.volume * 100)}%
          </label>
          <input
            type="range"
            min={0}
            max={2}
            step={0.01}
            value={clip.volume}
            onChange={(e) =>
              setClipVolume(trackId, clip.id, parseFloat(e.target.value))
            }
            className="w-full"
          />
        </div>
        {clip.media?.path && (clip.kind === "Audio" || clip.kind === "Video") && (
          <LoudnessMeter mediaPath={clip.media.path} />
        )}
        <EffectList clip={clip} trackId={trackId} />
      </div>
    </div>
  );
}
