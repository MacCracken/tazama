import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import { Modal } from "../shared/Modal";
import { ExportProgress as ExportProgressBar } from "./ExportProgress";
import * as commands from "../../ipc/commands";
import type { ExportFormat, ExportConfig } from "../../types";

export function ExportDialog() {
  const project = useProjectStore((s) => s.project);
  const setShow = useUIStore((s) => s.setShowExportDialog);
  const showToast = useUIStore((s) => s.showToast);
  const [format, setFormat] = useState<ExportFormat>("Mp4");
  const [exporting, setExporting] = useState(false);

  if (!project) return null;

  const handleExport = async () => {
    try {
      const ext = format === "Mp4" ? "mp4" : "webm";
      const path = await save({
        filters: [{ name: format, extensions: [ext] }],
        defaultPath: `${project.name}.${ext}`,
      });
      if (!path) return;

      const config: ExportConfig = {
        output_path: path,
        format,
        width: project.settings.width,
        height: project.settings.height,
        frame_rate: [
          project.settings.frame_rate.numerator,
          project.settings.frame_rate.denominator,
        ],
        sample_rate: project.settings.sample_rate,
        channels: project.settings.channels,
        hardware_accel: false,
      };

      setExporting(true);
      await commands.exportProject(project, config);
      setExporting(false);
      setShow(false);
      showToast("Export complete!", "success");
    } catch (e) {
      setExporting(false);
      showToast(String(e), "error");
    }
  };

  return (
    <Modal onClose={() => !exporting && setShow(false)} title="Export">
      {exporting ? (
        <ExportProgressBar />
      ) : (
        <div className="space-y-3">
          <div>
            <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
              Format
            </label>
            <div className="flex gap-1">
              {(["Mp4", "WebM"] as ExportFormat[]).map((f) => (
                <button
                  key={f}
                  onClick={() => setFormat(f)}
                  className="px-3 py-1 rounded text-xs"
                  style={{
                    background: format === f ? "var(--accent-primary)" : "var(--bg-hover)",
                    color: format === f ? "#fff" : "var(--text-secondary)",
                  }}
                >
                  {f}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
              Resolution
            </label>
            <div className="text-xs" style={{ color: "var(--text-primary)" }}>
              {project.settings.width} x {project.settings.height}
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
              onClick={handleExport}
              disabled={exporting}
              className="px-3 py-1 rounded text-xs font-medium"
              style={{
                background: exporting ? "var(--bg-hover)" : "var(--accent-primary)",
                color: "#fff",
                opacity: exporting ? 0.5 : 1,
              }}
            >
              Export
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
