import { usePlaybackStore } from "../../stores/playbackStore";

export function TransportControls() {
  const { isPlaying, togglePlayPause, stop, stepBack, stepForward } =
    usePlaybackStore();

  return (
    <div className="flex items-center gap-1">
      <button
        onClick={stepBack}
        className="p-1.5 rounded hover:bg-[var(--bg-hover)]"
        title="Step Back (Left Arrow)"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <rect x="2" y="3" width="2" height="10" />
          <path d="M12 3L6 8l6 5V3z" />
        </svg>
      </button>
      <button
        onClick={stop}
        className="p-1.5 rounded hover:bg-[var(--bg-hover)]"
        title="Stop"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <rect x="3" y="3" width="10" height="10" rx="1" />
        </svg>
      </button>
      <button
        onClick={togglePlayPause}
        className="p-1.5 rounded hover:bg-[var(--bg-hover)]"
        title="Play/Pause (Space)"
        style={{ color: isPlaying ? "var(--accent-primary)" : undefined }}
      >
        {isPlaying ? (
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="3" width="3" height="10" rx="1" />
            <rect x="10" y="3" width="3" height="10" rx="1" />
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 2l10 6-10 6V2z" />
          </svg>
        )}
      </button>
      <button
        onClick={stepForward}
        className="p-1.5 rounded hover:bg-[var(--bg-hover)]"
        title="Step Forward (Right Arrow)"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M4 3l6 5-6 5V3z" />
          <rect x="12" y="3" width="2" height="10" />
        </svg>
      </button>
    </div>
  );
}
