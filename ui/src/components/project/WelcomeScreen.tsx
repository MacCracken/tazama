import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import { open } from "@tauri-apps/plugin-dialog";
import { NewProjectDialog } from "./NewProjectDialog";

export function WelcomeScreen() {
  const openProject = useProjectStore((s) => s.openProject);
  const recentProjects = useProjectStore((s) => s.recentProjects);
  const showNewProjectDialog = useUIStore((s) => s.showNewProjectDialog);
  const setShowNewProjectDialog = useUIStore((s) => s.setShowNewProjectDialog);
  const showToast = useUIStore((s) => s.showToast);

  const handleOpen = async () => {
    try {
      const selected = await open({
        filters: [{ name: "Tazama Project", extensions: ["tazama"] }],
      });
      if (selected) await openProject(selected);
    } catch (e) {
      showToast(String(e), "error");
    }
  };

  return (
    <div
      className="flex flex-col items-center justify-center h-full gap-6"
      style={{ background: "var(--bg-primary)" }}
    >
      <h1
        className="text-2xl font-bold tracking-tight"
        style={{ color: "var(--text-primary)" }}
      >
        Tazama
      </h1>
      <p className="text-sm" style={{ color: "var(--text-muted)" }}>
        AI-native video editor
      </p>
      <div className="flex gap-3">
        <button
          onClick={() => setShowNewProjectDialog(true)}
          className="px-4 py-2 rounded text-sm font-medium"
          style={{
            background: "var(--accent-primary)",
            color: "#fff",
          }}
        >
          New Project
        </button>
        <button
          onClick={handleOpen}
          className="px-4 py-2 rounded text-sm"
          style={{
            background: "var(--bg-tertiary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-default)",
          }}
        >
          Open Project
        </button>
      </div>
      {recentProjects.length > 0 && (
        <div className="mt-4">
          <h3
            className="text-xs font-medium mb-2"
            style={{ color: "var(--text-muted)" }}
          >
            Recent Projects
          </h3>
          <div className="space-y-1">
            {recentProjects.map((path) => (
              <button
                key={path}
                onClick={() => openProject(path)}
                className="block w-full text-left px-3 py-1 rounded text-xs hover:bg-[var(--bg-hover)] truncate"
                style={{ color: "var(--text-secondary)", maxWidth: 300 }}
              >
                {path}
              </button>
            ))}
          </div>
        </div>
      )}
      {showNewProjectDialog && <NewProjectDialog />}
    </div>
  );
}
