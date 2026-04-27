use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    path::Path,
    ptr::null,
};

/// Copies a C string returned from OBS into an owned [`CString`].
/// Returns `None` if `ptr` is null.
///
/// # Safety
/// `ptr` must be null or point to a valid nul-terminated C string that
/// remains live for the duration of the call.
pub unsafe fn cstring_from_ptr(ptr: *const c_char) -> Option<CString> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr) }.to_owned())
    }
}

/// Copies bytes into a [`CString`], silently stripping any interior nul bytes
/// (`\0`). C strings cannot contain interior nuls, and the conversion would
/// otherwise have to fail. If you need to detect interior nuls, do that
/// check before calling this.
pub fn cstring_strip_nuls(bytes: impl Into<Vec<u8>>) -> CString {
    let mut bytes: Vec<u8> = bytes.into();
    bytes.retain(|&b| b != 0);
    CString::new(bytes).expect("interior nuls were stripped")
}

/// Lossily converts a path into a [`CString`]. Non-UTF-8 sequences become
/// U+FFFD and any interior nul bytes are stripped (see [`cstring_strip_nuls`]).
pub fn cstring_from_path(p: &Path) -> CString {
    cstring_strip_nuls(p.to_string_lossy().into_owned())
}

/// Returns the C pointer for `opt`, or null when `None`.
pub fn ptr_or_null(opt: Option<&CStr>) -> *const c_char {
    opt.map_or(null(), CStr::as_ptr)
}
