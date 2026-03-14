export function NoSelection() {
  return (
    <div className="flex flex-col h-full">
      <div
        className="px-2 py-1.5 border-b"
        style={{
          borderColor: "var(--border-default)",
          background: "var(--bg-tertiary)",
        }}
      >
        <span className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
          Inspector
        </span>
      </div>
      <div
        className="flex-1 flex items-center justify-center text-xs"
        style={{ color: "var(--text-muted)" }}
      >
        Select a clip or track
      </div>
    </div>
  );
}
