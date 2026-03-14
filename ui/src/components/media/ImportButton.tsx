import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import * as commands from "../../ipc/commands";

export function ImportButton() {
  const addMediaAsset = useProjectStore((s) => s.addMediaAsset);
  const project = useProjectStore((s) => s.project);
  const showToast = useUIStore((s) => s.showToast);

  const handleImport = async () => {
    if (!project) return;
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Media",
            extensions: [
              "mp4", "mkv", "webm", "mov", "avi",
              "mp3", "wav", "flac", "ogg", "aac",
              "png", "jpg", "jpeg", "bmp", "gif",
            ],
          },
        ],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      for (const path of paths) {
        try {
          const info = await commands.probeMedia(path);
          const name = path.split(/[\\/]/).pop() ?? path;
          addMediaAsset({
            path,
            name,
            duration_frames: info.duration_frames,
          });
        } catch (e) {
          showToast(`Failed to import ${path}: ${e}`, "error");
        }
      }
    } catch (e) {
      showToast(String(e), "error");
    }
  };

  return (
    <button
      onClick={handleImport}
      className="text-[10px] px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)]"
      style={{ color: "var(--text-accent)" }}
      disabled={!project}
    >
      + Import
    </button>
  );
}
