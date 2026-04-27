use std::ffi::CStr;

use crate::data::DataObj;
use crate::properties::Properties;
use crate::source::traits::CreateError;

use super::context::{
    CreatableEncoderContext, EncodeError, EncodeStatus, EncoderFrame, EncoderPacket,
};
use super::{EncoderRef, EncoderType};

/// Required trait for any encoder. Identifies the encoder, names the codec,
/// and constructs the per-instance state.
pub trait Encodable: Sized {
    /// Unique encoder id (`"obs_x264"`, `"my_h264_sei"`, ...). Registered into
    /// a global table.
    fn get_id() -> &'static CStr;

    /// Codec the encoder produces (`"h264"`, `"hevc"`, `"av1"`, `"aac"`,
    /// `"opus"`).
    fn get_codec() -> &'static CStr;

    fn get_type() -> EncoderType;

    fn create(
        ctx: &mut CreatableEncoderContext<Self>,
        encoder: EncoderRef,
    ) -> Result<Self, CreateError>;
}

pub trait GetNameEncoder {
    fn get_name() -> &'static CStr;
}

pub trait UpdateEncoder: Sized {
    /// Apply a new settings object. Return `true` on success.
    fn update(&mut self, settings: &mut DataObj) -> bool;
}

pub trait GetPropertiesEncoder: Sized {
    fn get_properties(&self) -> Properties;
}

pub trait GetDefaultsEncoder {
    fn get_defaults(settings: &mut DataObj);
}

/// CPU-path encode callback. Implement this for software encoders. Mutually
/// exclusive with [`EncodeTextureEncoder`] (a single encoder builder enables
/// only one).
pub trait EncodeEncoder: Sized {
    fn encode(
        &mut self,
        frame: &EncoderFrame<'_>,
        packet: &mut EncoderPacket<'_>,
    ) -> Result<EncodeStatus, EncodeError>;
}

/// GPU-path encode callback for encoders advertising
/// `EncoderCap::PassTexture`. Receives a shared-texture handle (Windows) or a
/// driver-specific texture id; the encoder is expected to consume it through
/// its native API.
pub trait EncodeTextureEncoder: Sized {
    fn encode_texture(
        &mut self,
        handle: u32,
        pts: i64,
        lock_key: u64,
        next_key: &mut u64,
        packet: &mut EncoderPacket<'_>,
    ) -> Result<EncodeStatus, EncodeError>;
}

pub trait GetExtraDataEncoder: Sized {
    /// Append codec-extradata (SPS/PPS, AAC config, etc.) into `out` and
    /// return `true`. Return `false` if no extra data is available.
    fn get_extra_data(&mut self, out: &mut Vec<u8>) -> bool;
}

pub trait GetSeiDataEncoder: Sized {
    /// Append SEI payload bytes into `out` and return `true`. Return `false`
    /// if no SEI is available.
    fn get_sei_data(&mut self, out: &mut Vec<u8>) -> bool;
}

/// Audio-only: number of audio frames the encoder consumes per call.
pub trait GetFrameSizeEncoder: Sized {
    fn get_frame_size(&self) -> usize;
}
