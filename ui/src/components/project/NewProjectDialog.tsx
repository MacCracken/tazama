import { useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import { Modal } from "../shared/Modal";

const presets = [
  { label: "1080p", width: 1920, height: 1080 },
  { label: "4K", width: 3840, height: 2160 },
  { label: "720p", width: 1280, height: 720 },
  { label: "Square", width: 1080, height: 1080 },
  { label: "Vertical", width: 1080, height: 1920 },
];

export function NewProjectDialog() {
  const createProject = useProjectStore((s) => s.createProject);
  const setShow = useUIStore((s) => s.setShowNewProjectDialog);
  const showToast = useUIStore((s) => s.showToast);
  const [name, setName] = useState("Untitled");
  const [width, setWidth] = useState(1920);
  const [height, setHeight] = useState(1080);

  const handleCreate = async () => {
    try {
      await createProject(name, width, height);
      setShow(false);
    } catch (e) {
      showToast(String(e), "error");
    }
  };

  return (
    <Modal onClose={() => setShow(false)} title="New Project">
      <div className="space-y-3">
        <div>
          <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
            Name
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full px-2 py-1 rounded text-xs"
            style={{
              background: "var(--bg-primary)",
              border: "1px solid var(--border-default)",
            }}
            autoFocus
          />
        </div>
        <div>
          <label className="block text-[10px] mb-1" style={{ color: "var(--text-muted)" }}>
            Resolution Preset
          </label>
          <div className="flex gap-1 flex-wrap">
            {presets.map((p) => (
              <button
                key={p.label}
                onClick={() => {
                  setWidth(p.width);
                  setHeight(p.height);
                }}
                className="px-2 py-0.5 rounded text-[10px]"
                style={{
                  background:
                    width === p.width && height === p.height
                      ? "var(--accent-primary)"
                      : "var(--bg-hover)",
                  color:
                    width === p.width && height === p.height
                      ? "#fff"
                      : "var(--text-secondary)",
                }}
              >
                {p.label}
              </button>
            ))}
          </div>
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
              Width
            </label>
            <input
              type="number"
              value={width}
              onChange={(e) => setWidth(parseInt(e.target.value) || 0)}
              className="w-full px-2 py-1 rounded text-xs"
              style={{
                background: "var(--bg-primary)",
                border: "1px solid var(--border-default)",
              }}
            />
          </div>
          <div>
            <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
              Height
            </label>
            <input
              type="number"
              value={height}
              onChange={(e) => setHeight(parseInt(e.target.value) || 0)}
              className="w-full px-2 py-1 rounded text-xs"
              style={{
                background: "var(--bg-primary)",
                border: "1px solid var(--border-default)",
              }}
            />
          </div>
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <button
            onClick={() => setShow(false)}
            className="px-3 py-1 rounded text-xs"
            style={{
              background: "var(--bg-hover)",
              color: "var(--text-secondary)",
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleCreate}
            className="px-3 py-1 rounded text-xs font-medium"
            style={{
              background: "var(--accent-primary)",
              color: "#fff",
            }}
          >
            Create
          </button>
        </div>
      </div>
    </Modal>
  );
}
