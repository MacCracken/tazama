import { useProjectStore } from "../../stores/projectStore";
import { ImportButton } from "./ImportButton";
import { MediaItem } from "./MediaItem";

export function MediaBrowser() {
  const mediaAssets = useProjectStore((s) => s.mediaAssets);

  return (
    <div className="flex flex-col h-full">
      <div
        className="flex items-center justify-between px-2 py-1.5 border-b"
        style={{
          borderColor: "var(--border-default)",
          background: "var(--bg-tertiary)",
        }}
      >
        <span className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
          Media
        </span>
        <ImportButton />
      </div>
      <div className="flex-1 overflow-y-auto p-1">
        {mediaAssets.map((asset) => (
          <MediaItem key={asset.path} asset={asset} />
        ))}
        {mediaAssets.length === 0 && (
          <div
            className="flex items-center justify-center h-20 text-xs"
            style={{ color: "var(--text-muted)" }}
          >
            Import media to begin
          </div>
        )}
      </div>
    </div>
  );
}
