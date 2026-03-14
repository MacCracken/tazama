use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::decode::AudioBuffer;

struct PreviewState {
    buffer: VecDeque<f32>,
    position_ns: u64,
    sample_rate: u32,
    channels: u16,
    playing: bool,
}

/// Audio preview via CPAL (auto-detects PipeWire via ALSA plugin layer).
pub struct AudioPreview {
    _stream: Stream,
    state: Arc<Mutex<PreviewState>>,
}

impl AudioPreview {
    /// Create a new audio preview output.
    ///
    /// Opens the default audio device and starts a stream. Audio data is fed
    /// via the `feed()` method; the CPAL callback reads from a ring buffer.
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self, cpal::BuildStreamError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let state = Arc::new(Mutex::new(PreviewState {
            buffer: VecDeque::with_capacity(sample_rate as usize * channels as usize),
            position_ns: 0,
            sample_rate,
            channels,
            playing: false,
        }));

        let cb_state = Arc::clone(&state);
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut st = cb_state.lock().unwrap();
                if !st.playing {
                    data.fill(0.0);
                    return;
                }
                for sample in data.iter_mut() {
                    *sample = st.buffer.pop_front().unwrap_or(0.0);
                }
                // Advance position based on samples consumed
                let frames_consumed = data.len() as u64 / st.channels as u64;
                let ns_per_frame = 1_000_000_000u64 / st.sample_rate as u64;
                st.position_ns += frames_consumed * ns_per_frame;
            },
            |err| {
                tracing::error!("CPAL stream error: {err}");
            },
            None,
        )?;

        stream.play().ok();

        Ok(Self {
            _stream: stream,
            state,
        })
    }

    /// Feed decoded audio samples into the ring buffer.
    pub fn feed(&self, audio: &AudioBuffer) {
        let mut st = self.state.lock().unwrap();
        st.buffer.extend(audio.samples.iter());
    }

    /// Seek to a position, clearing the buffer.
    pub fn seek(&self, position_ns: u64) {
        let mut st = self.state.lock().unwrap();
        st.buffer.clear();
        st.position_ns = position_ns;
    }

    /// Set playing state.
    pub fn set_playing(&self, playing: bool) {
        let mut st = self.state.lock().unwrap();
        st.playing = playing;
        if !playing {
            st.buffer.clear();
        }
    }

    /// Get the current playback position in nanoseconds.
    pub fn position_ns(&self) -> u64 {
        self.state.lock().unwrap().position_ns
    }
}
