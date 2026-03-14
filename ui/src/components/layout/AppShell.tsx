import { Toolbar } from "../toolbar/Toolbar";
import { MediaBrowser } from "../media/MediaBrowser";
import { PreviewMonitor } from "../preview/PreviewMonitor";
import { Inspector } from "../inspector/Inspector";
import { TimelinePanel } from "../timeline/TimelinePanel";
import { useUIStore } from "../../stores/uiStore";

export function AppShell() {
  const panelSizes = useUIStore((s) => s.panelSizes);

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
        className="border-t"
        style={{
          height: `${panelSizes.timeline}%`,
          minHeight: 150,
          borderColor: "var(--border-default)",
          background: "var(--bg-secondary)",
        }}
      >
        <TimelinePanel />
      </div>
    </div>
  );
}
