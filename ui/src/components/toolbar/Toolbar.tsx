import { FileActions } from "./FileActions";
import { EditTools } from "./EditTools";
import { TransportControls } from "./TransportControls";
import { TimeDisplay } from "./TimeDisplay";
import { useUIStore } from "../../stores/uiStore";

export function Toolbar() {
  const showMixer = useUIStore((s) => s.showMixer);
  const toggleMixer = useUIStore((s) => s.toggleMixer);

  return (
    <div
      className="flex items-center gap-2 px-2 flex-shrink-0 border-b"
      style={{
        height: "var(--toolbar-height)",
        background: "var(--bg-tertiary)",
        borderColor: "var(--border-default)",
      }}
    >
      <FileActions />
      <div className="w-px h-5" style={{ background: "var(--border-default)" }} />
      <EditTools />
      <div className="flex-1" />
      <TransportControls />
      <TimeDisplay />
      <div className="w-px h-5" style={{ background: "var(--border-default)" }} />
      <button
        onClick={toggleMixer}
        className="text-[10px] px-2 py-1 rounded"
        style={{
          background: showMixer ? "var(--accent-primary)" : "var(--bg-hover)",
          color: showMixer ? "#fff" : "var(--text-secondary)",
        }}
        title="Toggle Mixer"
      >
        Mixer
      </button>
    </div>
  );
}
