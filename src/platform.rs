//! Safe wrappers over libobs's `util/platform.h` helpers.
//!
//! `obs-sys-rs` binds only `<obs-module.h>` and `<obs-frontend-api.h>`,
//! which do not include `util/platform.h`; the few helpers plugins need
//! are declared here against the symbols libobs exports.

unsafe extern "C" {
    fn os_gettime_ns() -> u64;
}

/// Returns libobs's monotonic clock reading, in nanoseconds.
///
/// This is `os_gettime_ns`, the clock domain of OBS frame timestamps and
/// encoder `sys_dts_usec` values. It is not wall-clock time.
pub fn monotonic_ns() -> u64 {
    unsafe { os_gettime_ns() }
}
