import { useCallback, useRef, useEffect } from "react";

interface PanelResizerProps {
  direction: "horizontal" | "vertical";
  onResize: (delta: number) => void;
}

export function PanelResizer({ direction, onResize }: PanelResizerProps) {
  const dragging = useRef(false);
  const lastPos = useRef(0);
  const handlersRef = useRef<{
    move: ((e: MouseEvent) => void) | null;
    up: (() => void) | null;
  }>({ move: null, up: null });

  // Clean up any active drag listeners on unmount
  useEffect(() => {
    return () => {
      if (handlersRef.current.move) {
        document.removeEventListener("mousemove", handlersRef.current.move);
      }
      if (handlersRef.current.up) {
        document.removeEventListener("mouseup", handlersRef.current.up);
      }
    };
  }, []);

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
        handlersRef.current = { move: null, up: null };
      };

      handlersRef.current = { move: handleMouseMove, up: handleMouseUp };
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
