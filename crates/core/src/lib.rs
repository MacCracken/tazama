pub mod clip;
pub mod command;
pub mod effect;
pub mod keyframe;
pub mod marker;
pub mod media_info;
pub mod multicam;
pub mod playback;
pub mod plugin;
pub mod project;
pub mod timeline;

pub use clip::{Clip, ClipId, ClipKind, MediaRef};
pub use command::{EditCommand, EditHistory};
pub use effect::{Effect, EffectId, EffectKind, TransitionKind};
pub use keyframe::{Interpolation, Keyframe, KeyframeId, KeyframeTrack, KeyframeTrackId};
pub use marker::{Marker, MarkerColor, MarkerId};
pub use media_info::{
    AudioStreamInfo, Codec, ContainerFormat, MediaInfo, ThumbnailSpec, ThumbnailStrategy,
    VideoStreamInfo, WaveformData,
};
pub use multicam::{MultiCamGroup, MultiCamGroupId};
pub use playback::{PlaybackPosition, PlaybackState};
pub use plugin::{PluginEffectDef, PluginManifest, PluginParamDef};
pub use project::{FrameRate, Project, ProjectId, ProjectSettings};
pub use timeline::{Timeline, TimelineError, Track, TrackId, TrackKind};
