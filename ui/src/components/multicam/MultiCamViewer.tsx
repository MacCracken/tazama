import { useProjectStore } from "../../stores/projectStore";
import type { MultiCamGroup } from "../../types";

interface MultiCamViewerProps {
  group: MultiCamGroup;
  currentFrame: number;
  onSwitchAngle: (angleIndex: number) => void;
}

export function MultiCamViewer({
  group,
  currentFrame,
  onSwitchAngle,
}: MultiCamViewerProps) {
  const project = useProjectStore((s) => s.project);
  if (!project) return null;

  return (
    <div className="multicam-viewer">
      <div className="multicam-header">{group.name}</div>
      <div className="multicam-grid">
        {group.angles.map(([trackId, syncOffset], index) => {
          const track = project.timeline.tracks.find((t) => t.id === trackId);
          const trackName = track?.name ?? `Angle ${index + 1}`;

          return (
            <div
              key={trackId}
              className="multicam-angle"
              onClick={() => onSwitchAngle(index)}
            >
              <div className="angle-label">{trackName}</div>
              <div className="angle-preview">
                {/* Preview thumbnail would go here */}
                <span className="angle-frame">
                  F{currentFrame + syncOffset}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
