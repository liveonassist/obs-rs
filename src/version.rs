use crate::obs_sys_rs::{
    LIBOBS_API_MAJOR_VER, LIBOBS_API_MINOR_VER, LIBOBS_API_PATCH_VER, obs_get_version,
};
use std::fmt;

/// A libobs version, `MAJOR.MINOR.PATCH`.
///
/// A plugin sees two: [`Version::COMPILED`], the headers it was built
/// against, and [`Version::running`], the libobs that loaded it, which may be
/// newer.
///
/// ```
/// use obs_rs::version::Version;
///
/// let version = Version::new(31, 1, 2);
/// assert_eq!(version.to_string(), "31.1.2");
/// assert_eq!(Version::from_packed(version.to_packed()), version);
/// assert!(Version::new(31, 10, 0) > Version::new(31, 9, 0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// The libobs headers this crate was compiled against (`LIBOBS_API_VER`).
    pub const COMPILED: Self = Self::new(
        LIBOBS_API_MAJOR_VER,
        LIBOBS_API_MINOR_VER,
        LIBOBS_API_PATCH_VER,
    );

    /// Builds a version from its parts.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// The libobs the plugin is running in (`obs_get_version`).
    ///
    /// Callable from any thread, before or after OBS starts up.
    pub fn running() -> Self {
        // SAFETY: `obs_get_version` takes no arguments, touches no state and
        // returns a constant compiled into libobs.
        Self::from_packed(unsafe { obs_get_version() })
    }

    /// Unpacks libobs's `MAKE_SEMANTIC_VERSION` form: major in the top 8
    /// bits, minor in the next 8 and patch in the low 16.
    pub const fn from_packed(packed: u32) -> Self {
        Self::new(packed >> 24, (packed >> 16) & 0xff, packed & 0xffff)
    }

    /// Packs into libobs's `MAKE_SEMANTIC_VERSION` form, as returned by
    /// `obs_get_version` and expected from `obs_module_ver`.
    pub const fn to_packed(self) -> u32 {
        (self.major << 24) | (self.minor << 16) | self.patch
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
