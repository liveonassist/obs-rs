//! Encoder API.
//!
//! Mirror of `src/source/`: the user implements [`Encodable`] (mandatory) plus
//! whichever per-callback traits they need, then opts each into the
//! registration via [`EncoderInfoBuilder::enable_*`].
//!
//! ```ignore
//! impl Encodable for MyH264 {
//!     fn get_id() -> &'static CStr { c"my_h264" }
//!     fn get_codec() -> &'static CStr { c"h264" }
//!     fn get_type() -> EncoderType { EncoderType::Video }
//!     fn create(ctx: &mut CreatableEncoderContext<Self>, encoder: EncoderRef)
//!         -> Result<Self, CreateError>
//!     { /* … */ }
//! }
//! impl GetNameEncoder for MyH264 { /* … */ }
//! impl EncodeEncoder for MyH264 { /* … */ }
//!
//! load_context.register_encoder(
//!     load_context
//!         .create_encoder_builder::<MyH264>()
//!         .enable_get_name()
//!         .enable_encode()
//!         .with_caps(EncoderCap::PassTexture | EncoderCap::DynBitrate)
//!         .build(),
//! );
//! ```

pub mod context;
mod ffi;
pub mod traits;

use std::marker::PhantomData;

use enumflags2::{BitFlags, bitflags};
use obs_rs_sys::{
    OBS_ENCODER_CAP_DEPRECATED, OBS_ENCODER_CAP_DYN_BITRATE, OBS_ENCODER_CAP_INTERNAL,
    OBS_ENCODER_CAP_PASS_TEXTURE, OBS_ENCODER_CAP_ROI, obs_encoder_get_codec,
    obs_encoder_get_height, obs_encoder_get_id, obs_encoder_get_name, obs_encoder_get_ref,
    obs_encoder_get_width, obs_encoder_info, obs_encoder_release, obs_encoder_t, obs_encoder_type,
    obs_encoder_type_OBS_ENCODER_AUDIO, obs_encoder_type_OBS_ENCODER_VIDEO,
};
#[cfg(any(feature = "obs-31", feature = "obs-32"))]
use obs_rs_sys::OBS_ENCODER_CAP_SCALING;
use paste::item;

use std::ffi::CString;

use crate::media::{audio::AudioRef, video::VideoRef};
use crate::string::cstring_from_ptr;
use crate::wrapper::PtrWrapper;
use crate::{Error, Result};

pub use context::*;
pub use traits::*;

/// Encoder kind. Maps to libobs's `obs_encoder_type`.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EncoderType {
    Audio,
    Video,
}

impl EncoderType {
    pub fn as_raw(self) -> obs_encoder_type {
        match self {
            EncoderType::Audio => obs_encoder_type_OBS_ENCODER_AUDIO,
            EncoderType::Video => obs_encoder_type_OBS_ENCODER_VIDEO,
        }
    }
}

/// Encoder capability flags. OR them with `|` to compose; pass to
/// [`EncoderInfoBuilder::with_caps`].
//
// Split into per-version enum bodies because `enumflags2`'s `#[bitflags]`
// attribute does not propagate `#[cfg]` on individual variants — the impl
// code it expands references every variant unconditionally. Keeping the cfg
// at item level avoids that.
#[cfg(feature = "obs-30")]
#[bitflags]
#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EncoderCap {
    Deprecated = OBS_ENCODER_CAP_DEPRECATED,
    PassTexture = OBS_ENCODER_CAP_PASS_TEXTURE,
    DynBitrate = OBS_ENCODER_CAP_DYN_BITRATE,
    Internal = OBS_ENCODER_CAP_INTERNAL,
    Roi = OBS_ENCODER_CAP_ROI,
}

#[cfg(any(feature = "obs-31", feature = "obs-32"))]
#[bitflags]
#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EncoderCap {
    Deprecated = OBS_ENCODER_CAP_DEPRECATED,
    PassTexture = OBS_ENCODER_CAP_PASS_TEXTURE,
    DynBitrate = OBS_ENCODER_CAP_DYN_BITRATE,
    Internal = OBS_ENCODER_CAP_INTERNAL,
    Roi = OBS_ENCODER_CAP_ROI,
    Scaling = OBS_ENCODER_CAP_SCALING,
}

/// Reference-counted handle to an `obs_encoder_t`.
pub struct EncoderRef {
    inner: *mut obs_encoder_t,
}

impl_ptr_wrapper!(
    @ptr: inner,
    EncoderRef,
    obs_encoder_t,
    obs_encoder_get_ref,
    obs_encoder_release
);

impl EncoderRef {
    pub fn name(&self) -> Result<CString> {
        unsafe { cstring_from_ptr(obs_encoder_get_name(self.inner)) }
            .ok_or(Error::NulPointer("obs_encoder_get_name"))
    }

    pub fn id(&self) -> Result<CString> {
        unsafe { cstring_from_ptr(obs_encoder_get_id(self.inner)) }
            .ok_or(Error::NulPointer("obs_encoder_get_id"))
    }

    pub fn codec(&self) -> Result<CString> {
        unsafe { cstring_from_ptr(obs_encoder_get_codec(self.inner)) }
            .ok_or(Error::NulPointer("obs_encoder_get_codec"))
    }

    pub fn width(&self) -> u32 {
        unsafe { obs_encoder_get_width(self.inner) }
    }

    pub fn height(&self) -> u32 {
        unsafe { obs_encoder_get_height(self.inner) }
    }

    /// Output `video_t` this encoder is bound to (only meaningful for video
    /// encoders that have been associated with a video output).
    pub fn video(&self) -> Option<VideoRef> {
        let ptr = unsafe { obs_rs_sys::obs_encoder_video(self.inner) };
        if ptr.is_null() {
            None
        } else {
            Some(VideoRef::from_raw(ptr))
        }
    }

    /// Output `audio_t` this encoder is bound to (audio encoders).
    pub fn audio(&self) -> Option<AudioRef> {
        let ptr = unsafe { obs_rs_sys::obs_encoder_audio(self.inner) };
        if ptr.is_null() {
            None
        } else {
            Some(AudioRef::from_raw(ptr))
        }
    }
}

/// Boxed `obs_encoder_info` produced by [`EncoderInfoBuilder::build`]. Hand
/// to [`crate::module::LoadContext::register_encoder`].
pub struct EncoderInfo {
    info: Box<obs_encoder_info>,
}

impl EncoderInfo {
    /// # Safety
    /// Transfers ownership of the heap allocation. Caller (typically the
    /// `LoadContext`) must reclaim it via `Box::from_raw` at unload.
    pub unsafe fn into_raw(self) -> *mut obs_encoder_info {
        Box::into_raw(self.info)
    }
}

impl AsRef<obs_encoder_info> for EncoderInfo {
    fn as_ref(&self) -> &obs_encoder_info {
        self.info.as_ref()
    }
}

/// Builder for [`obs_encoder_info`]. Each `enable_*` method is gated on the
/// matching trait being implemented for `D`.
pub struct EncoderInfoBuilder<D: Encodable> {
    __data: PhantomData<D>,
    info: obs_encoder_info,
}

impl<D: Encodable> EncoderInfoBuilder<D> {
    pub(crate) fn new() -> Self {
        Self {
            __data: PhantomData,
            info: obs_encoder_info {
                id: D::get_id().as_ptr(),
                type_: D::get_type().as_raw(),
                codec: D::get_codec().as_ptr(),
                create: Some(ffi::create::<D>),
                destroy: Some(ffi::destroy::<D>),
                ..Default::default()
            },
        }
    }

    /// Set the encoder's capability flag set.
    pub fn with_caps(mut self, caps: BitFlags<EncoderCap>) -> Self {
        self.info.caps = caps.bits();
        self
    }

    pub fn build(self) -> EncoderInfo {
        // Sanity: every encoder must produce packets via *some* encode path.
        debug_assert!(
            self.info.encode.is_some()
                || self.info.encode_texture.is_some()
                || self.info.encode_texture2.is_some(),
            "encoder `{}` has no encode callback — call .enable_encode() or .enable_encode_texture()",
            D::get_id().to_string_lossy(),
        );
        if D::get_type() == EncoderType::Audio {
            debug_assert!(
                self.info.get_frame_size.is_some(),
                "audio encoder `{}` must implement GetFrameSizeEncoder",
                D::get_id().to_string_lossy(),
            );
        }

        EncoderInfo {
            info: Box::new(self.info),
        }
    }
}

macro_rules! impl_encoder_builder {
    ($($f:ident => $t:ident)*) => ($(
        item! {
            impl<D: Encodable + [<$t>]> EncoderInfoBuilder<D> {
                pub fn [<enable_$f>](mut self) -> Self {
                    self.info.[<$f>] = Some(ffi::[<$f>]::<D>);
                    self
                }
            }
        }
    )*)
}

impl_encoder_builder! {
    get_name => GetNameEncoder
    encode => EncodeEncoder
    encode_texture => EncodeTextureEncoder
    update => UpdateEncoder
    get_defaults => GetDefaultsEncoder
    get_properties => GetPropertiesEncoder
    get_extra_data => GetExtraDataEncoder
    get_sei_data => GetSeiDataEncoder
    get_frame_size => GetFrameSizeEncoder
}

/// Re-export so callers can write `EncoderCaps::empty()` / type annotations
/// without depending on `enumflags2` directly. `EncoderCap | EncoderCap`
/// already produces a `BitFlags<EncoderCap>` via the `bitor` impl.
pub type EncoderCaps = BitFlags<EncoderCap>;
