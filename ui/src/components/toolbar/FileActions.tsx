import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";

export function FileActions() {
  const { saveProject, openProject, filePath, project } = useProjectStore();
  const { saveProjectAs } = useProjectStore();
  const setShowNewProjectDialog = useUIStore((s) => s.setShowNewProjectDialog);
  const setShowExportDialog = useUIStore((s) => s.setShowExportDialog);
  const showToast = useUIStore((s) => s.showToast);
  const [loading, setLoading] = useState(false);

  const handleNew = () => setShowNewProjectDialog(true);

  const handleOpen = async () => {
    try {
      setLoading(true);
      const selected = await open({
        filters: [{ name: "Tazama Project", extensions: ["tazama"] }],
      });
      if (selected) {
        await openProject(selected);
      }
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    try {
      setLoading(true);
      if (filePath) {
        await saveProject();
      } else {
        const path = await save({
          filters: [{ name: "Tazama Project", extensions: ["tazama"] }],
        });
        if (path) {
          await saveProjectAs(path);
        }
      }
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setLoading(false);
    }
  };

  const handleExport = () => setShowExportDialog(true);

  return (
    <div className="flex items-center gap-0.5">
      <button
        onClick={handleNew}
        className="px-2 py-1 rounded text-xs hover:bg-[var(--bg-hover)]"
        style={{ color: "var(--text-secondary)" }}
        title="New Project (Ctrl+N)"
        disabled={loading}
      >
        New
      </button>
      <button
        onClick={handleOpen}
        className="px-2 py-1 rounded text-xs hover:bg-[var(--bg-hover)]"
        style={{ color: "var(--text-secondary)", opacity: loading ? 0.5 : 1 }}
        title="Open Project (Ctrl+O)"
        disabled={loading}
      >
        {loading ? "Loading..." : "Open"}
      </button>
      <button
        onClick={handleSave}
        className="px-2 py-1 rounded text-xs hover:bg-[var(--bg-hover)]"
        style={{ color: "var(--text-secondary)" }}
        title="Save Project (Ctrl+S)"
        disabled={!project || loading}
      >
        Save
      </button>
      <button
        onClick={handleExport}
        className="px-2 py-1 rounded text-xs hover:bg-[var(--bg-hover)]"
        style={{ color: "var(--text-secondary)" }}
        title="Export (Ctrl+E)"
        disabled={!project || loading}
      >
        Export
      </button>
    </div>
  );
}
