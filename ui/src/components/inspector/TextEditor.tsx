import { useState } from "react";
import type { Effect } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

interface TextEditorProps {
  trackId: string;
  clipId: string;
  effect?: Effect;
}

export function TextEditor({ trackId, clipId, effect }: TextEditorProps) {
  const addEffect = useProjectStore((s) => s.addEffect);

  const existing =
    effect && "Text" in effect.kind ? effect.kind.Text : null;

  const [content, setContent] = useState(existing?.content ?? "");
  const [fontFamily, setFontFamily] = useState(existing?.font_family ?? "Arial");
  const [fontSize, setFontSize] = useState(existing?.font_size ?? 48);
  const [x, setX] = useState(existing?.x ?? 100);
  const [y, setY] = useState(existing?.y ?? 100);

  const handleApply = () => {
    const textEffect: Effect = {
      id: effect?.id ?? crypto.randomUUID(),
      kind: {
        Text: {
          content,
          font_family: fontFamily,
          font_size: fontSize,
          color: [1.0, 1.0, 1.0, 1.0],
          x,
          y,
        },
      },
      enabled: true,
      keyframe_tracks: effect?.keyframe_tracks ?? [],
    };
    addEffect(trackId, clipId, textEffect);
  };

  return (
    <div className="text-editor">
      <div className="text-field">
        <label>Text</label>
        <input
          type="text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
        />
      </div>
      <div className="text-field">
        <label>Font</label>
        <input
          type="text"
          value={fontFamily}
          onChange={(e) => setFontFamily(e.target.value)}
        />
      </div>
      <div className="text-field">
        <label>Size</label>
        <input
          type="number"
          value={fontSize}
          min={1}
          max={500}
          onChange={(e) => setFontSize(parseFloat(e.target.value))}
        />
      </div>
      <div className="text-field">
        <label>X</label>
        <input
          type="number"
          value={x}
          onChange={(e) => setX(parseFloat(e.target.value))}
        />
      </div>
      <div className="text-field">
        <label>Y</label>
        <input
          type="number"
          value={y}
          onChange={(e) => setY(parseFloat(e.target.value))}
        />
      </div>
      <button onClick={handleApply}>Apply Text</button>
    </div>
  );
}
