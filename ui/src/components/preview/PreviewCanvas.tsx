import { useRef, useEffect, useCallback } from "react";
import { usePlaybackStore } from "../../stores/playbackStore";
import { useProjectStore } from "../../stores/projectStore";
import * as commands from "../../ipc/commands";

export function PreviewCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const position = usePlaybackStore((s) => s.position);
  const project = useProjectStore((s) => s.project);
  const pendingRef = useRef(false);
  const lastRenderedRef = useRef(-1);

  const renderFrame = useCallback(
    async (frameIndex: number) => {
      if (!project || pendingRef.current) return;
      if (frameIndex === lastRenderedRef.current) return;

      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      pendingRef.current = true;
      try {
        const frame = await commands.renderPreviewFrame(project, frameIndex);
        lastRenderedRef.current = frameIndex;

        // Decode base64 RGBA data and draw to canvas
        const binary = atob(frame.data);
        const bytes = new Uint8ClampedArray(binary.length);
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i);
        }

        // Update canvas dimensions if needed
        if (canvas.width !== frame.width || canvas.height !== frame.height) {
          canvas.width = frame.width;
          canvas.height = frame.height;
        }

        const imageData = new ImageData(bytes, frame.width, frame.height);
        ctx.putImageData(imageData, 0, 0);
      } catch {
        // Render error — show black frame (e.g. no clip at this position)
        const ctx2 = canvas.getContext("2d");
        if (ctx2) {
          ctx2.fillStyle = "#000";
          ctx2.fillRect(0, 0, canvas.width, canvas.height);
        }
      } finally {
        pendingRef.current = false;
      }
    },
    [project],
  );

  // Render on position change
  useEffect(() => {
    renderFrame(position);
  }, [position, renderFrame]);

  // Clear canvas when no project
  useEffect(() => {
    if (project) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    lastRenderedRef.current = -1;
  }, [project]);

  return (
    <canvas
      ref={canvasRef}
      width={1920}
      height={1080}
      className="absolute inset-0 w-full h-full rounded"
      style={{ background: "#000" }}
    />
  );
}
