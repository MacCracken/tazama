import { useCallback, useRef } from "react";

interface PanelResizerProps {
  direction: "horizontal" | "vertical";
  onResize: (delta: number) => void;
}

export function PanelResizer({ direction, onResize }: PanelResizerProps) {
  const dragging = useRef(false);
  const lastPos = useRef(0);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = true;
      lastPos.current = direction === "horizontal" ? e.clientX : e.clientY;

      const handleMouseMove = (e: MouseEvent) => {
        if (!dragging.current) return;
        const current = direction === "horizontal" ? e.clientX : e.clientY;
        const delta = current - lastPos.current;
        lastPos.current = current;
        onResize(delta);
      };

      const handleMouseUp = () => {
        dragging.current = false;
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [direction, onResize],
  );

  return (
    <div
      onMouseDown={handleMouseDown}
      className={
        direction === "horizontal"
          ? "w-1 cursor-col-resize hover:bg-[var(--accent-primary)] transition-colors"
          : "h-1 cursor-row-resize hover:bg-[var(--accent-primary)] transition-colors"
      }
      style={{ background: "var(--border-default)" }}
    />
  );
}
