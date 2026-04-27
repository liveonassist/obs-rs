//! Audio and video media types.
//!
//! Re-exports the audio, video, and media-state submodules. The
//! [`AudioRef`](crate::media::audio::AudioRef) and
//! [`VideoRef`](crate::media::video::VideoRef) handles correspond to
//! libobs's global audio and video outputs; the `*DataContext` and
//! `*Source*` types provide borrowed views over the per-frame buffers
//! OBS hands to source and output callbacks.

pub mod audio;
pub mod state;
pub mod video;

pub use audio::*;
pub use state::*;
pub use video::*;
