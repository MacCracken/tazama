use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    H264,
    H265,
    Vp9,
    Av1,
    Aac,
    Opus,
    Flac,
    Mp3,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerFormat {
    Mp4,
    Mkv,
    WebM,
    Mov,
    Avi,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamInfo {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub frame_rate: (u32, u32),
    pub bit_depth: u32,
    pub pixel_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamInfo {
    pub codec: Codec,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub duration_ms: u64,
    pub duration_frames: u64,
    pub container: ContainerFormat,
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformData {
    pub sample_rate: u32,
    pub channels: u16,
    pub peaks_per_second: u32,
    /// Per-channel peaks: `peaks[channel][sample] = (min, max)`.
    pub peaks: Vec<Vec<(f32, f32)>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThumbnailSpec {
    pub width: u32,
    pub height: u32,
    pub interval_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_info_serde_round_trip() {
        let info = MediaInfo {
            duration_ms: 5000,
            duration_frames: 150,
            container: ContainerFormat::Mp4,
            video_streams: vec![VideoStreamInfo {
                codec: Codec::H264,
                width: 1920,
                height: 1080,
                frame_rate: (30, 1),
                bit_depth: 8,
                pixel_format: "yuv420p".into(),
            }],
            audio_streams: vec![AudioStreamInfo {
                codec: Codec::Aac,
                sample_rate: 48000,
                channels: 2,
                bit_depth: 16,
            }],
            file_size: 1_000_000,
        };

        let json = serde_json::to_string(&info).unwrap();
        let round_tripped: MediaInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.duration_ms, info.duration_ms);
        assert_eq!(round_tripped.duration_frames, info.duration_frames);
        assert_eq!(round_tripped.video_streams.len(), 1);
        assert_eq!(round_tripped.audio_streams.len(), 1);
        assert_eq!(round_tripped.video_streams[0].codec, Codec::H264);
        assert_eq!(round_tripped.audio_streams[0].codec, Codec::Aac);
    }

    #[test]
    fn waveform_data_serde_round_trip() {
        let waveform = WaveformData {
            sample_rate: 48000,
            channels: 2,
            peaks_per_second: 100,
            peaks: vec![
                vec![(-0.5, 0.8), (-0.3, 0.6)],
                vec![(-0.4, 0.7), (-0.2, 0.5)],
            ],
        };

        let json = serde_json::to_string(&waveform).unwrap();
        let round_tripped: WaveformData = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.channels, 2);
        assert_eq!(round_tripped.peaks.len(), 2);
        assert_eq!(round_tripped.peaks[0][0], (-0.5, 0.8));
    }

    #[test]
    fn thumbnail_spec_serde_round_trip() {
        let spec = ThumbnailSpec {
            width: 320,
            height: 180,
            interval_ms: 1000,
        };

        let json = serde_json::to_string(&spec).unwrap();
        let round_tripped: ThumbnailSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.width, spec.width);
        assert_eq!(round_tripped.height, spec.height);
        assert_eq!(round_tripped.interval_ms, spec.interval_ms);
    }

    #[test]
    fn codec_variants_serde() {
        for codec in [
            Codec::H264,
            Codec::H265,
            Codec::Vp9,
            Codec::Av1,
            Codec::Aac,
            Codec::Opus,
            Codec::Flac,
            Codec::Mp3,
            Codec::Other,
        ] {
            let json = serde_json::to_string(&codec).unwrap();
            let round_tripped: Codec = serde_json::from_str(&json).unwrap();
            assert_eq!(round_tripped, codec);
        }
    }

    #[test]
    fn container_format_variants_serde() {
        for fmt in [
            ContainerFormat::Mp4,
            ContainerFormat::Mkv,
            ContainerFormat::WebM,
            ContainerFormat::Mov,
            ContainerFormat::Avi,
            ContainerFormat::Other,
        ] {
            let json = serde_json::to_string(&fmt).unwrap();
            let round_tripped: ContainerFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(round_tripped, fmt);
        }
    }
}
