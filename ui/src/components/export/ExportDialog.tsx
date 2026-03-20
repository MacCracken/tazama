import { useState, useEffect } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import { Modal } from "../shared/Modal";
import { ExportProgress as ExportProgressBar } from "./ExportProgress";
import * as commands from "../../ipc/commands";
import type { ExportFormat, ExportConfig, HardwareInfo } from "../../types";

function formatBytes(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(0)} MB`;
  return `${bytes} B`;
}

function HardwarePanel({ hardware }: { hardware: HardwareInfo[] }) {
  if (hardware.length === 0) return null;
  return (
    <div>
      <label className="block text-[10px] mb-0.5" style={{ color: "var(--text-muted)" }}>
        Hardware
      </label>
      <div className="space-y-1">
        {hardware.map((hw, i) => (
          <div
            key={i}
            className="flex items-center justify-between text-[10px] px-1.5 py-1 rounded"
            style={{ background: "var(--bg-primary)" }}
          >
            <span style={{ color: "var(--text-primary)" }}>{hw.family}</span>
            <span className="flex items-center gap-2" style={{ color: "var(--text-muted)" }}>
              {hw.memory_free_bytes != null && (
                <span>{formatBytes(hw.memory_free_bytes)} free</span>
              )}
              {hw.temperature_c != null && <span>{hw.temperature_c}°C</span>}
              {hw.gpu_utilization_percent != null && <span>{hw.gpu_utilization_percent}%</span>}
              {hw.memory_free_bytes == null &&
                hw.temperature_c == null &&
                hw.gpu_utilization_percent == null && (
                  <span>{formatBytes(hw.memory_bytes)}</span>
                )}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function ExportDialog() {
  const project = useProjectStore((s) => s.project);
  const setShow = useUIStore((s) => s.setShowExportDialog);
  const showToast = useUIStore((s) => s.showToast);
  const [format, setFormat] = useState<ExportFormat>("Mp4");
  const [exporting, setExporting] = useState(false);
  const [hardware, setHardware] = useState<HardwareInfo[]>([]);

  useEffect(() => {
    commands.detectHardware().then((result) => {
      setHardware(result.accelerators);
    }).catch(() => {});
  }, []);

  if (!project) return null;

  const handleExport = async () => {
    try {
      const extMap: Record<string, string> = { Mp4: "mp4", WebM: "webm", Mkv: "mkv" };
      const ext = extMap[format] ?? "mp4";
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
              {(["Mp4", "Mkv", "WebM"] as ExportFormat[]).map((f) => (
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
          <HardwarePanel hardware={hardware} />
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
