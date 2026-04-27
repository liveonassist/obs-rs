//! Common machinery for wrapping reference-counted OBS pointer types.
//!
//! libobs hands out a number of `obs_*_t` handle types whose lifetime is
//! managed by an associated pair of `*_get_ref` / `*_release` functions.
//! [`PtrWrapper`](crate::wrapper::PtrWrapper) models that ownership in Rust: implementors track an
//! owned reference and release it on `Drop`. The crate's pointer-wrapping
//! types ([`SourceRef`](crate::source::SourceRef),
//! [`OutputRef`](crate::output::OutputRef),
//! [`EncoderRef`](crate::encoder::EncoderRef), and similar) are produced
//! by the `impl_ptr_wrapper!` macro and all expose this trait.

use std::mem::forget;

/// Internal companion to [`PtrWrapper`] used by the `impl_ptr_wrapper!`
/// macro to bridge between the trait and the concrete struct field.
///
/// Implementors are produced by the macro; user code should not call
/// these methods directly.
pub trait PtrWrapperInternal: PtrWrapper {
    /// Constructs the wrapper from an already-owned pointer.
    ///
    /// # Safety
    ///
    /// Use [`PtrWrapper::from_raw`] or
    /// [`PtrWrapper::from_raw_unchecked`] instead — this method does not
    /// validate ownership.
    unsafe fn new_internal(ptr: *mut Self::Pointer) -> Self;

    /// Returns the inner pointer.
    ///
    /// # Safety
    ///
    /// Use [`PtrWrapper::as_ptr`], [`PtrWrapper::as_ptr_mut`], or
    /// [`PtrWrapper::into_raw`] instead.
    unsafe fn get_internal(&self) -> *mut Self::Pointer;
}

/// A safe wrapper around a reference-counted OBS pointer.
///
/// Implementors own a reference to a libobs handle and release it on
/// `Drop`. Concrete implementations are normally produced by the
/// `impl_ptr_wrapper!` macro.
pub trait PtrWrapper: Sized {
    /// The wrapped libobs handle type (e.g. `obs_source_t`).
    type Pointer;

    /// Increments the reference count on `ptr`.
    ///
    /// # Safety
    ///
    /// Calls into the underlying C API; intended for internal use by
    /// [`from_raw`](Self::from_raw).
    unsafe fn get_ref(ptr: *mut Self::Pointer) -> *mut Self::Pointer;

    /// Decrements the reference count on `ptr`.
    ///
    /// # Safety
    ///
    /// Calls into the underlying C API; intended for internal use by
    /// `Drop`.
    unsafe fn release(ptr: *mut Self::Pointer);

    /// Wraps `raw`, taking a fresh reference.
    ///
    /// Returns `None` if `raw` is null. The returned wrapper owns its
    /// reference and will release it on drop.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn from_raw(raw: *mut Self::Pointer) -> Option<Self> {
        unsafe { Self::from_raw_unchecked(Self::get_ref(raw)) }
    }

    /// Wraps an already-owned pointer without taking an additional
    /// reference.
    ///
    /// # Safety
    ///
    /// `raw` must be a pointer the caller already owns a reference to;
    /// the returned wrapper assumes that reference and will release it
    /// on drop.
    unsafe fn from_raw_unchecked(raw: *mut Self::Pointer) -> Option<Self>;

    /// Returns the inner pointer without transferring ownership.
    ///
    /// # Safety
    ///
    /// The returned pointer must only be used for the duration of the
    /// borrow of `self`, and must not be released by the caller.
    unsafe fn as_ptr(&self) -> *const Self::Pointer;

    /// Consumes the wrapper and returns the inner pointer, transferring
    /// ownership of the reference to the caller.
    ///
    /// `Drop` does not run on the consumed wrapper. The caller becomes
    /// responsible for eventually releasing the pointer.
    fn into_raw(self) -> *mut Self::Pointer {
        let raw = unsafe { self.as_ptr_mut() };
        forget(self);
        raw
    }

    /// Returns the inner pointer as `*mut`, without transferring
    /// ownership.
    ///
    /// # Safety
    ///
    /// Same constraints as [`as_ptr`](Self::as_ptr): the pointer must
    /// not outlive the borrow and must not be released by the caller.
    unsafe fn as_ptr_mut(&self) -> *mut Self::Pointer {
        self.as_ptr() as *mut _
    }
}

macro_rules! impl_ptr_wrapper {
    (@ptr: $field:ident, $ref:ty, $($tt:tt)*) => {
        impl_ptr_wrapper!(@__impl.trait.internal $ref, $field);
        impl_ptr_wrapper!($ref, $($tt)*);
    };
    ($ref:ty, $ptr:ty, @identity, $release:expr) => {
        impl_ptr_wrapper!(@__impl.trait.wrapper $ref, $ptr, impl_ptr_wrapper!{@__impl.fn.id}, $release);
        // when get_ref is `@identity`, no `Clone implemented`
        impl_ptr_wrapper!(@__impl.trait.drop $ref, $ptr);
    };
    ($ref:ty, $ptr:ty, $get_ref:expr, $release:expr) => {
        impl_ptr_wrapper!(@__impl.trait.wrapper $ref, $ptr, impl_ptr_wrapper!{@__impl.fn.get_ref $get_ref}, $release);
        impl_ptr_wrapper!(@__impl.trait.clone $ref, $ptr);
        impl_ptr_wrapper!(@__impl.trait.drop $ref, $ptr);
    };

    ($ref:ty, $ptr:ty, @addref: $add_ref:expr, $release:expr) => {
        impl_ptr_wrapper!(@__impl.trait.wrapper $ref, $ptr, impl_ptr_wrapper!{@__impl.fn.add_ref $add_ref}, $release);
        impl_ptr_wrapper!(@__impl.trait.clone $ref, $ptr);
        impl_ptr_wrapper!(@__impl.trait.drop $ref, $ptr);
    };
    (@__impl.fn.get_ref $get_ref:expr) => {
        unsafe fn get_ref(ptr: *mut Self::Pointer) -> *mut Self::Pointer {
            unsafe { $get_ref(ptr) }
        }
    };
    (@__impl.fn.add_ref $add_ref:expr) => {
        unsafe fn get_ref(ptr: *mut Self::Pointer) -> *mut Self::Pointer {
            unsafe { $add_ref(ptr); ptr }
        }
    };
    (@__impl.fn.id) => {
        unsafe fn get_ref(ptr: *mut Self::Pointer) -> *mut Self::Pointer {
            ptr
        }
    };
    (@__impl.trait.internal $ref:ty, $field:ident) => {
        impl $crate::wrapper::PtrWrapperInternal for $ref {
            unsafe fn new_internal(ptr: *mut Self::Pointer) -> Self {
                Self { $field: ptr }
            }
            unsafe fn get_internal(&self) -> *mut Self::Pointer {
                self.$field
            }
        }
    };
    (@__impl.trait.wrapper $ref:ty, $ptr:ty, $get_ref:item, $release:expr) => {
        impl $crate::wrapper::PtrWrapper for $ref {
            type Pointer = $ptr;

            unsafe fn from_raw_unchecked(raw: *mut Self::Pointer) -> Option<Self> {
                use $crate::wrapper::PtrWrapperInternal;
                if raw.is_null() {
                    None
                } else {
                    Some(Self::new_internal(raw))
                }
            }

            $get_ref

            unsafe fn release(ptr: *mut Self::Pointer) {
                unsafe { $release(ptr) }
            }

            unsafe fn as_ptr(&self) -> *const Self::Pointer {
                use $crate::wrapper::PtrWrapperInternal;
                self.get_internal()
            }
        }
    };
    (@__impl.trait.clone $ref:ty, $ptr:ty) => {
        impl Clone for $ref {
            fn clone(&self) -> Self {
                Self::from_raw(unsafe { self.as_ptr_mut() }).expect("clone")
            }
        }
    };
    (@__impl.trait.drop $ref:ty, $ptr:ty) => {
        impl Drop for $ref {
            fn drop(&mut self) {
                use $crate::wrapper::PtrWrapper;
                unsafe { Self::release(self.as_ptr_mut()) }
            }
        }
    };
}
