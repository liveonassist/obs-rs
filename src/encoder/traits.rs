use std::ffi::CStr;

use crate::data::DataObj;
use crate::properties::Properties;
use crate::source::traits::CreateError;

use super::context::{
    CreatableEncoderContext, EncodeError, EncodeStatus, EncoderFrame, EncoderPacket,
};
use super::{EncoderRef, EncoderType};

/// The mandatory trait every encoder must implement.
///
/// `Encodable` identifies the encoder, names the codec it produces, and
/// constructs the per-instance state. Implementations are paired with the
/// optional traits in this module by [`EncoderInfoBuilder`] at registration
/// time.
///
/// [`EncoderInfoBuilder`]: super::EncoderInfoBuilder
pub trait Encodable: Sized {
    /// Returns the globally-unique identifier for this encoder type, for
    /// example `c"obs_x264"` or `c"my_h264_sei"`.
    ///
    /// OBS records this id in a process-global table; it must be stable
    /// across plugin loads and unique among all registered encoders.
    fn get_id() -> &'static CStr;

    /// Returns the codec name produced by this encoder, for example
    /// `c"h264"`, `c"hevc"`, `c"av1"`, `c"aac"`, or `c"opus"`.
    fn get_codec() -> &'static CStr;

    /// Returns whether this encoder produces audio or video packets.
    fn get_type() -> EncoderType;

    /// Constructs a new encoder instance from the given settings.
    ///
    /// Called by OBS each time a new encoder of this type is created.
    /// `encoder` is a reference to the freshly-allocated `obs_encoder_t`
    /// the new instance is associated with.
    fn create(
        ctx: &mut CreatableEncoderContext<Self>,
        encoder: EncoderRef,
    ) -> Result<Self, CreateError>;
}

/// Provides a localized, user-visible display name for the encoder.
///
/// Enable with
/// [`EncoderInfoBuilder::enable_get_name`](super::EncoderInfoBuilder::enable_get_name).
pub trait GetNameEncoder {
    /// Returns the display name shown in OBS UIs.
    fn get_name() -> &'static CStr;
}

/// Applies a new settings object to a running encoder.
///
/// Enable with
/// [`EncoderInfoBuilder::enable_update`](super::EncoderInfoBuilder::enable_update).
pub trait UpdateEncoder: Sized {
    /// Applies updated settings. Returns `true` on success, `false` to
    /// indicate the encoder could not adopt the new settings.
    fn update(&mut self, settings: &mut DataObj) -> bool;
}

/// Builds the user-facing [`Properties`] panel for the encoder.
///
/// Enable with
/// [`EncoderInfoBuilder::enable_get_properties`](super::EncoderInfoBuilder::enable_get_properties).
pub trait GetPropertiesEncoder: Sized {
    /// Returns the property tree that OBS will render in the encoder's
    /// settings UI.
    fn get_properties(&self) -> Properties;
}

/// Populates the default settings written into a freshly-created encoder.
///
/// Enable with
/// [`EncoderInfoBuilder::enable_get_defaults`](super::EncoderInfoBuilder::enable_get_defaults).
pub trait GetDefaultsEncoder {
    /// Writes default values into `settings`.
    fn get_defaults(settings: &mut DataObj);
}

/// CPU-path encode callback for software encoders.
///
/// Mutually exclusive with [`EncodeTextureEncoder`]: a single encoder can
/// register only one encode path. Enable with
/// [`EncoderInfoBuilder::enable_encode`](super::EncoderInfoBuilder::enable_encode).
pub trait EncodeEncoder: Sized {
    /// Consumes a frame of input and, when ready, produces a packet.
    ///
    /// Return [`EncodeStatus::Received`] to deliver a packet, or
    /// [`EncodeStatus::NotReady`] if the encoder is still buffering.
    fn encode(
        &mut self,
        frame: &EncoderFrame<'_>,
        packet: &mut EncoderPacket<'_>,
    ) -> Result<EncodeStatus, EncodeError>;
}

/// GPU-path encode callback for encoders advertising
/// [`EncoderCap::PassTexture`](super::EncoderCap::PassTexture).
///
/// The callback receives a shared-texture handle (on Windows) or a
/// driver-specific texture identifier on other platforms; the encoder is
/// expected to consume it through its native API. Enable with
/// [`EncoderInfoBuilder::enable_encode_texture`](super::EncoderInfoBuilder::enable_encode_texture).
pub trait EncodeTextureEncoder: Sized {
    /// Consumes a GPU texture and, when ready, produces a packet.
    ///
    /// `lock_key` is the keyed-mutex value the texture was last released
    /// with; write the value to release the texture with into `next_key`.
    fn encode_texture(
        &mut self,
        handle: u32,
        pts: i64,
        lock_key: u64,
        next_key: &mut u64,
        packet: &mut EncoderPacket<'_>,
    ) -> Result<EncodeStatus, EncodeError>;
}

/// Provides codec-level out-of-band data (extradata).
///
/// Used for codec configuration blobs such as H.264 SPS/PPS, HEVC
/// VPS/SPS/PPS, or AAC AudioSpecificConfig. Enable with
/// [`EncoderInfoBuilder::enable_get_extra_data`](super::EncoderInfoBuilder::enable_get_extra_data).
pub trait GetExtraDataEncoder: Sized {
    /// Appends extradata bytes to `out` and returns `true`. Returns `false`
    /// if no extradata is available.
    fn get_extra_data(&mut self, out: &mut Vec<u8>) -> bool;
}

/// Provides SEI (Supplemental Enhancement Information) payload bytes.
///
/// Enable with
/// [`EncoderInfoBuilder::enable_get_sei_data`](super::EncoderInfoBuilder::enable_get_sei_data).
pub trait GetSeiDataEncoder: Sized {
    /// Appends SEI payload bytes to `out` and returns `true`. Returns
    /// `false` if no SEI is available.
    fn get_sei_data(&mut self, out: &mut Vec<u8>) -> bool;
}

/// Reports the audio frame size consumed per encode call.
///
/// Required for audio encoders. Enable with
/// [`EncoderInfoBuilder::enable_get_frame_size`](super::EncoderInfoBuilder::enable_get_frame_size).
pub trait GetFrameSizeEncoder: Sized {
    /// Returns the number of audio frames the encoder consumes per call.
    fn get_frame_size(&self) -> usize;
}
