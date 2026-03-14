import { FileActions } from "./FileActions";
import { EditTools } from "./EditTools";
import { TransportControls } from "./TransportControls";
import { TimeDisplay } from "./TimeDisplay";

export function Toolbar() {
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
    </div>
  );
}
