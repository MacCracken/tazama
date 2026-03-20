import { useState, useCallback } from "react";
import * as commands from "../../ipc/commands";

interface LoudnessMeterProps {
  mediaPath: string;
}

export function LoudnessMeter({ mediaPath }: LoudnessMeterProps) {
  const [lufs, setLufs] = useState<number | null>(null);
  const [measuring, setMeasuring] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleMeasure = useCallback(async () => {
    setMeasuring(true);
    setError(null);
    try {
      const result = await commands.measureLoudness(mediaPath);
      setLufs(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setMeasuring(false);
    }
  }, [mediaPath]);

  // Visual indicator: green (-14 to -10), yellow (-23 to -14), red (below -23)
  const lufsColor = lufs !== null
    ? lufs > -10 ? "var(--error)" : lufs > -23 ? "var(--text-accent)" : "var(--text-muted)"
    : "var(--text-muted)";

  return (
    <div>
      <div className="flex items-center justify-between mb-0.5">
        <label className="text-[10px]" style={{ color: "var(--text-muted)" }}>
          Loudness
        </label>
        <button
          onClick={handleMeasure}
          disabled={measuring}
          className="text-[10px] px-1 rounded hover:bg-[var(--bg-hover)]"
          style={{
            color: "var(--text-accent)",
            opacity: measuring ? 0.5 : 1,
          }}
        >
          {measuring ? "..." : "Measure"}
        </button>
      </div>
      {lufs !== null && (
        <div className="flex items-baseline gap-1">
          <span className="text-xs font-mono font-medium" style={{ color: lufsColor }}>
            {lufs.toFixed(1)}
          </span>
          <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>LUFS</span>
        </div>
      )}
      {error && (
        <div className="text-[10px]" style={{ color: "var(--error)" }}>
          {error}
        </div>
      )}
    </div>
  );
}
