import { useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import { ImportButton } from "./ImportButton";
import { MediaItem } from "./MediaItem";
import * as commands from "../../ipc/commands";
import type { ThumbnailStrategy } from "../../types";

export function MediaBrowser() {
  const project = useProjectStore((s) => s.project);
  const mediaAssets = useProjectStore((s) => s.mediaAssets);
  const thumbnailStrategy = useUIStore((s) => s.thumbnailStrategy);
  const setThumbnailStrategy = useUIStore((s) => s.setThumbnailStrategy);
  const showToast = useUIStore((s) => s.showToast);
  const [generatingProxies, setGeneratingProxies] = useState(false);
  const [proxyMode, setProxyMode] = useState(false);

  const handleGenerateProxies = async () => {
    if (!project) return;
    setGeneratingProxies(true);
    try {
      const proxyDir = "/tmp/tazama-proxies";
      const paths = await commands.generateProxies(project, proxyDir, 640);
      showToast(`Generated ${paths.length} proxy file${paths.length !== 1 ? "s" : ""}`, "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setGeneratingProxies(false);
    }
  };

  const handleToggleProxyMode = async () => {
    const next = !proxyMode;
    setProxyMode(next);
    await commands.setProxyMode(next).catch(() => {});
  };

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
      <div className="flex items-center gap-1 px-2 py-1 border-b" style={{ borderColor: "var(--border-default)" }}>
        <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>Thumbnails:</span>
        {(["SceneBased", "ContentBased"] as ThumbnailStrategy[]).map((s) => (
          <button
            key={s}
            onClick={() => setThumbnailStrategy(s)}
            className="text-[10px] px-1.5 py-0.5 rounded"
            style={{
              background: thumbnailStrategy === s ? "var(--accent-primary)" : "var(--bg-hover)",
              color: thumbnailStrategy === s ? "#fff" : "var(--text-muted)",
            }}
          >
            {s === "SceneBased" ? "Scene" : "Content"}
          </button>
        ))}
      </div>
      {mediaAssets.length > 0 && (
        <div className="flex items-center gap-1 px-2 py-1 border-b" style={{ borderColor: "var(--border-default)" }}>
          <button
            onClick={handleGenerateProxies}
            disabled={generatingProxies || !project}
            className="text-[10px] px-1.5 py-0.5 rounded"
            style={{
              background: "var(--bg-hover)",
              color: generatingProxies ? "var(--text-muted)" : "var(--text-accent)",
              opacity: generatingProxies ? 0.5 : 1,
            }}
          >
            {generatingProxies ? "Generating..." : "Generate Proxies"}
          </button>
          <button
            onClick={handleToggleProxyMode}
            className="text-[10px] px-1.5 py-0.5 rounded"
            style={{
              background: proxyMode ? "var(--accent-primary)" : "var(--bg-hover)",
              color: proxyMode ? "#fff" : "var(--text-muted)",
            }}
          >
            Proxy
          </button>
        </div>
      )}
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
