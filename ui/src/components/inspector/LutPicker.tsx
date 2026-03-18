import type { Effect } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

interface LutPickerProps {
  trackId: string;
  clipId: string;
}

export function LutPicker({ trackId, clipId }: LutPickerProps) {
  const addEffect = useProjectStore((s) => s.addEffect);

  const handleSelectLut = async () => {
    // In a full implementation, this would open a native file dialog
    // via Tauri's dialog plugin to select a .cube file
    const lutPath = prompt("Enter path to .cube LUT file:");
    if (!lutPath) return;

    const effect: Effect = {
      id: crypto.randomUUID(),
      kind: { Lut: { lut_path: lutPath } },
      enabled: true,
      keyframe_tracks: [],
    };
    addEffect(trackId, clipId, effect);
  };

  return (
    <div className="lut-picker">
      <button onClick={handleSelectLut}>Import LUT (.cube)</button>
    </div>
  );
}
