import { useEffect } from "react";
import { useUIStore } from "../../stores/uiStore";

export function Toast() {
  const message = useUIStore((s) => s.toastMessage);
  const type = useUIStore((s) => s.toastType);
  const clearToast = useUIStore((s) => s.clearToast);

  useEffect(() => {
    if (!message) return;
    const timer = setTimeout(clearToast, 4000);
    return () => clearTimeout(timer);
  }, [message, clearToast]);

  if (!message) return null;

  const colors = {
    error: "var(--error)",
    success: "var(--success)",
    info: "var(--accent-primary)",
  };

  return (
    <div
      className="fixed bottom-4 right-4 z-50 px-4 py-2 rounded-lg shadow-lg text-xs max-w-sm"
      style={{
        background: "var(--bg-elevated)",
        border: `1px solid ${colors[type]}`,
        color: "var(--text-primary)",
      }}
    >
      {message.split("\n").map((line, i) => (
        <div key={i}>{line}</div>
      ))}
    </div>
  );
}
