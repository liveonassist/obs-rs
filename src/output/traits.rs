use std::ffi::CStr;

use crate::encoder::context::EncodedPacketView;
use crate::media::{audio::AudioDataOutputContext, video::VideoDataOutputContext};
use crate::{prelude::DataObj, properties::Properties};

use super::{CreatableOutputContext, OutputRef};
use crate::source::traits::CreateError;

/// The mandatory trait every output must implement.
///
/// `Outputable` identifies the output, constructs its per-instance state,
/// and provides the lifecycle hooks OBS calls when the output is started
/// or stopped. Implementations are paired with the optional traits in this
/// module by [`OutputInfoBuilder`].
///
/// [`OutputInfoBuilder`]: super::OutputInfoBuilder
pub trait Outputable: Sized {
    /// Returns the globally-unique identifier for this output type.
    ///
    /// OBS records this id in a process-global table; it must be stable
    /// across plugin loads and unique among all registered outputs.
    fn get_id() -> &'static CStr;

    /// Constructs a new output instance.
    ///
    /// Called by OBS each time a new output of this type is created.
    fn create(
        context: &mut CreatableOutputContext<'_, Self>,
        output: OutputRef,
    ) -> Result<Self, CreateError>;

    /// Begins delivery. Returns `true` to indicate that the output started
    /// successfully, or `false` to abort the start.
    ///
    /// The default implementation always returns `true`.
    fn start(&mut self) -> bool {
        true
    }

    /// Ends delivery. `ts` is the wall-clock timestamp at which the stop
    /// was requested.
    ///
    /// The default implementation does nothing.
    fn stop(&mut self, _ts: u64) {}
}

/// Provides a localized, user-visible display name for the output.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_name`](super::OutputInfoBuilder::enable_get_name).
pub trait GetNameOutput {
    /// Returns the display name shown in OBS UIs.
    fn get_name() -> &'static CStr;
}

/// Receives raw video frames from OBS.
///
/// Enable with
/// [`OutputInfoBuilder::enable_raw_video`](super::OutputInfoBuilder::enable_raw_video).
pub trait RawVideoOutput: Sized {
    /// Called for each raw video frame produced by the bound video output.
    fn raw_video(&mut self, frame: &mut VideoDataOutputContext);
}

/// Receives raw audio buffers from OBS on a single track.
///
/// Enable with
/// [`OutputInfoBuilder::enable_raw_audio`](super::OutputInfoBuilder::enable_raw_audio).
pub trait RawAudioOutput: Sized {
    /// Called for each raw audio buffer produced by the bound audio output.
    fn raw_audio(&mut self, frame: &mut AudioDataOutputContext);
}

/// Receives raw audio buffers from OBS on multiple tracks.
///
/// Implementing this trait marks the output as multi-track. Enable with
/// [`OutputInfoBuilder::enable_raw_audio2`](super::OutputInfoBuilder::enable_raw_audio2).
pub trait RawAudio2Output: Sized {
    /// Called for each raw audio buffer on track `idx`.
    fn raw_audio2(&mut self, idx: usize, frame: &mut AudioDataOutputContext);
}

/// Receives pre-encoded packets from upstream encoders.
///
/// Enable with
/// [`OutputInfoBuilder::enable_encoded_packet`](super::OutputInfoBuilder::enable_encoded_packet).
pub trait EncodedPacketOutput: Sized {
    /// Called for each encoded packet routed to this output.
    fn encoded_packet(&mut self, packet: &EncodedPacketView<'_>);
}

/// Applies a new settings object to a running output.
///
/// Enable with
/// [`OutputInfoBuilder::enable_update`](super::OutputInfoBuilder::enable_update).
pub trait UpdateOutput: Sized {
    /// Applies updated settings.
    fn update(&mut self, settings: &mut DataObj);
}

/// Populates the default settings written into a freshly-created output.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_defaults`](super::OutputInfoBuilder::enable_get_defaults).
pub trait GetDefaultsOutput {
    /// Writes default values into `settings`.
    fn get_defaults(settings: &mut DataObj);
}

/// Builds the user-facing [`Properties`] panel for the output.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_properties`](super::OutputInfoBuilder::enable_get_properties).
pub trait GetPropertiesOutput: Sized {
    /// Returns the property tree that OBS will render in the output's
    /// settings UI.
    fn get_properties(&self) -> Properties;
}

/// Reports the total number of bytes this output has delivered.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_total_bytes`](super::OutputInfoBuilder::enable_get_total_bytes).
pub trait GetTotalBytesOutput: Sized {
    /// Returns the cumulative number of bytes sent or written.
    fn get_total_bytes(&self) -> u64;
}

/// Reports the number of frames this output has dropped.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_dropped_frames`](super::OutputInfoBuilder::enable_get_dropped_frames).
pub trait GetDroppedFramesOutput: Sized {
    /// Returns the cumulative number of dropped frames.
    fn get_dropped_frames(&self) -> i32;
}

/// Reports the current congestion level on the output, in the range
/// `0.0..=1.0`, where `1.0` indicates the output is fully congested.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_congestion`](super::OutputInfoBuilder::enable_get_congestion).
pub trait GetCongestionOutput: Sized {
    /// Returns the current congestion estimate.
    fn get_congestion(&self) -> f32;
}

/// Reports the time taken to establish the output's connection.
///
/// Enable with
/// [`OutputInfoBuilder::enable_get_connect_time_ms`](super::OutputInfoBuilder::enable_get_connect_time_ms).
pub trait GetConnectTimeMsOutput: Sized {
    /// Returns the connection time, in milliseconds.
    fn get_connect_time_ms(&self) -> i32;
}
