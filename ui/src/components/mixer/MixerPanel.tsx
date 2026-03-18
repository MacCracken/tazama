import { useProjectStore } from "../../stores/projectStore";

export function MixerPanel() {
  const project = useProjectStore((s) => s.project);
  const setTrackVolume = useProjectStore((s) => s.setTrackVolume);
  const setTrackPan = useProjectStore((s) => s.setTrackPan);
  const toggleTrackMute = useProjectStore((s) => s.toggleTrackMute);
  const toggleTrackSolo = useProjectStore((s) => s.toggleTrackSolo);

  if (!project) return null;

  const tracks = project.timeline.tracks;

  return (
    <div className="mixer-panel">
      <div className="mixer-header">Mixer</div>
      <div className="mixer-tracks">
        {tracks.map((track) => (
          <div key={track.id} className="mixer-track">
            <div className="mixer-track-name">{track.name}</div>

            <div className="mixer-fader">
              <label>Vol</label>
              <input
                type="range"
                min={0}
                max={2}
                step={0.01}
                value={track.volume}
                onChange={(e) =>
                  setTrackVolume(track.id, parseFloat(e.target.value))
                }
              />
              <span>{Math.round(track.volume * 100)}%</span>
            </div>

            <div className="mixer-pan">
              <label>Pan</label>
              <input
                type="range"
                min={-1}
                max={1}
                step={0.01}
                value={track.pan}
                onChange={(e) =>
                  setTrackPan(track.id, parseFloat(e.target.value))
                }
              />
              <span>{track.pan > 0 ? `R${Math.round(track.pan * 100)}` : track.pan < 0 ? `L${Math.round(-track.pan * 100)}` : "C"}</span>
            </div>

            <div className="mixer-buttons">
              <button
                className={track.muted ? "active" : ""}
                onClick={() => toggleTrackMute(track.id)}
              >
                M
              </button>
              <button
                className={track.solo ? "active" : ""}
                onClick={() => toggleTrackSolo(track.id)}
              >
                S
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
