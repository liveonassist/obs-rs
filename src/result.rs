/// A specialized [`Result`](core::result::Result) type for `obs-rs`
/// operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The error type returned by fallible operations in `obs-rs`.
///
/// Most variants surface a specific failure mode of the underlying OBS
/// API; the embedded `&'static str` typically names the libobs function
/// whose return value triggered the error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An OBS API call failed with the given numeric error code.
    #[error("OBS Error: {0}")]
    ObsError(i32),

    /// An OBS API returned an unexpected null pointer. The string
    /// identifies the call site.
    #[error("Null Pointer Error: {0}")]
    NulPointer(&'static str),

    /// A Rust string contained an interior nul byte and could not be
    /// converted to a `CString`.
    #[error("Null String Error: {0}")]
    NulError(#[from] std::ffi::NulError),

    /// A C string returned by OBS was not valid UTF-8.
    #[error("Utf8 Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    /// An OBS enum value did not correspond to a known variant. The
    /// string is the enum name and the integer is the offending value.
    #[error("Enum Out of Range: {0} {1}")]
    EnumOutOfRange(&'static str, i64),

    /// A filesystem path was not valid UTF-8.
    #[error("Path Error: utf8")]
    PathUtf8,
}

/// Extension methods for converting [`Option`] values into `obs-rs`
/// [`Result`]s.
pub trait OptionExt {
    /// Maps `None` to [`Error::NulPointer`] with the given context string.
    fn null_pointer(self, msg: &'static str) -> Result<()>;
}

impl OptionExt for Option<()> {
    fn null_pointer(self, msg: &'static str) -> Result<()> {
        self.ok_or(Error::NulPointer(msg))
    }
}
