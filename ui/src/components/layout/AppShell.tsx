import { Toolbar } from "../toolbar/Toolbar";
import { MediaBrowser } from "../media/MediaBrowser";
import { PreviewMonitor } from "../preview/PreviewMonitor";
import { Inspector } from "../inspector/Inspector";
import { TimelinePanel } from "../timeline/TimelinePanel";
import { MixerPanel } from "../mixer/MixerPanel";
import { useUIStore } from "../../stores/uiStore";

export function AppShell() {
  const panelSizes = useUIStore((s) => s.panelSizes);
  const showMixer = useUIStore((s) => s.showMixer);
  const toggleMixer = useUIStore((s) => s.toggleMixer);

  return (
    <div className="flex flex-col h-full">
      <Toolbar />
      <div className="flex flex-1 min-h-0">
        <div
          className="flex-shrink-0 border-r"
          style={{
            width: panelSizes.mediaBrowser,
            borderColor: "var(--border-default)",
            background: "var(--bg-secondary)",
          }}
        >
          <MediaBrowser />
        </div>
        <div className="flex-1 min-w-0" style={{ background: "var(--bg-primary)" }}>
          <PreviewMonitor />
        </div>
        <div
          className="flex-shrink-0 border-l"
          style={{
            width: panelSizes.inspector,
            borderColor: "var(--border-default)",
            background: "var(--bg-secondary)",
          }}
        >
          <Inspector />
        </div>
      </div>
      <div
        className="border-t flex"
        style={{
          height: `${panelSizes.timeline}%`,
          minHeight: 150,
          borderColor: "var(--border-default)",
          background: "var(--bg-secondary)",
        }}
      >
        <div className="flex-1 min-w-0 flex flex-col">
          <TimelinePanel />
        </div>
        {showMixer && (
          <div
            className="flex-shrink-0 border-l"
            style={{
              width: 280,
              borderColor: "var(--border-default)",
              background: "var(--bg-tertiary)",
            }}
          >
            <div
              className="flex items-center justify-between px-2 py-1 border-b"
              style={{ borderColor: "var(--border-default)" }}
            >
              <span className="text-[10px] font-medium" style={{ color: "var(--text-secondary)" }}>
                Mixer
              </span>
              <button
                onClick={toggleMixer}
                className="text-[10px] px-1 rounded hover:bg-[var(--bg-hover)]"
                style={{ color: "var(--text-muted)" }}
              >
                x
              </button>
            </div>
            <div style={{ height: "calc(100% - 28px)" }}>
              <MixerPanel />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
