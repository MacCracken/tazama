import { useEffect, useRef } from "react";
import type { Clip } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import { usePlaybackStore } from "../../stores/playbackStore";

interface ClipContextMenuProps {
  clip: Clip;
  trackId: string;
  x: number;
  y: number;
  onClose: () => void;
}

export function ClipContextMenu({ clip, trackId, x, y, onClose }: ClipContextMenuProps) {
  const removeClip = useProjectStore((s) => s.removeClip);
  const splitClip = useProjectStore((s) => s.splitClip);
  const addClip = useProjectStore((s) => s.addClip);
  const position = usePlaybackStore((s) => s.position);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const escHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handler);
    document.addEventListener("keydown", escHandler);
    return () => {
      document.removeEventListener("mousedown", handler);
      document.removeEventListener("keydown", escHandler);
    };
  }, [onClose]);

  const canSplit =
    position > clip.timeline_start &&
    position < clip.timeline_start + clip.duration;

  const handleSplit = () => {
    if (canSplit) splitClip(trackId, clip.id, position);
    onClose();
  };

  const handleDelete = () => {
    removeClip(trackId, clip.id);
    onClose();
  };

  const handleDuplicate = () => {
    const dup: Clip = {
      ...clip,
      id: crypto.randomUUID(),
      name: `${clip.name} (copy)`,
      timeline_start: clip.timeline_start + clip.duration,
    };
    addClip(trackId, dup);
    onClose();
  };

  const items = [
    { label: "Split at Playhead", action: handleSplit, disabled: !canSplit },
    { label: "Duplicate", action: handleDuplicate, disabled: false },
    { label: "Delete", action: handleDelete, disabled: false, danger: true },
  ];

  return (
    <div
      ref={menuRef}
      className="fixed z-[100] rounded shadow-lg py-1 min-w-[140px]"
      style={{
        left: x,
        top: y,
        background: "var(--bg-secondary)",
        border: "1px solid var(--border-default)",
      }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          onClick={item.action}
          disabled={item.disabled}
          className="block w-full text-left px-3 py-1 text-[11px] hover:bg-[var(--bg-hover)] disabled:opacity-30 disabled:cursor-default"
          style={{
            color: item.danger ? "var(--error)" : "var(--text-primary)",
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
