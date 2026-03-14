import React from "react";
import { useUIStore } from "../../stores/uiStore";
import type { ActiveTool } from "../../types";

const tools: { id: ActiveTool; label: string; shortcut: string; icon: React.ReactNode }[] = [
  {
    id: "select",
    label: "Select",
    shortcut: "V",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M3 1l10 6.5L8 9l-1.5 5.5L3 1z" />
      </svg>
    ),
  },
  {
    id: "razor",
    label: "Razor",
    shortcut: "B",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 1v14M4 4l4-3 4 3" />
        <rect x="7" y="1" width="2" height="14" />
      </svg>
    ),
  },
  {
    id: "slip",
    label: "Slip",
    shortcut: "S",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M2 8h12M5 5l-3 3 3 3M11 5l3 3-3 3" />
      </svg>
    ),
  },
];

export function EditTools() {
  const activeTool = useUIStore((s) => s.activeTool);
  const setActiveTool = useUIStore((s) => s.setActiveTool);

  return (
    <div className="flex items-center gap-0.5">
      {tools.map((tool) => (
        <button
          key={tool.id}
          onClick={() => setActiveTool(tool.id)}
          className="p-1.5 rounded"
          style={{
            background:
              activeTool === tool.id ? "var(--bg-active)" : undefined,
            color:
              activeTool === tool.id
                ? "var(--text-accent)"
                : "var(--text-secondary)",
          }}
          title={`${tool.label} (${tool.shortcut})`}
        >
          {tool.icon}
        </button>
      ))}
    </div>
  );
}
