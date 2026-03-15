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
      let succeeded = 0;
      const failures: { name: string; error: string }[] = [];

      for (const path of paths) {
        const fileName = path.split(/[\\/]/).pop() ?? path;
        try {
          const info = await commands.probeMedia(path);
          // Copy file into the project media directory
          const projectRoot = project.name;
          const importedPath = await commands.importMedia(projectRoot, path);
          addMediaAsset({
            path: importedPath,
            name: fileName,
            duration_frames: info.duration_frames,
          });
          succeeded++;
        } catch (e) {
          failures.push({ name: fileName, error: String(e) });
        }
      }

      // Show summary
      if (failures.length === 0) {
        if (succeeded > 1) {
          showToast(`Imported ${succeeded} files`, "success");
        }
      } else if (succeeded === 0) {
        const detail = failures.map((f) => `${f.name}: ${f.error}`).join("\n");
        showToast(`All ${failures.length} imports failed:\n${detail}`, "error");
      } else {
        const detail = failures.map((f) => `${f.name}: ${f.error}`).join("\n");
        showToast(
          `Imported ${succeeded} of ${paths.length} files. Failed:\n${detail}`,
          "error",
        );
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
