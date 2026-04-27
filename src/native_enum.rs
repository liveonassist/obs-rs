/// The error returned when an integer cannot be matched to any variant
/// of an `obs-rs` enum mirror.
///
/// Produced by the `from_raw` helper that the [`native_enum!`] macro
/// generates for each enum.
#[derive(Debug)]
pub struct NativeParsingError {
    struct_name: &'static str,
    value: i64,
}

impl NativeParsingError {
    pub(crate) fn new(struct_name: &'static str, value: i64) -> Self {
        Self { struct_name, value }
    }
}

impl std::fmt::Display for NativeParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to convert native value {} into {}",
            self.value, self.struct_name
        )
    }
}

impl std::error::Error for NativeParsingError {}

/// Defines a Rust enum that mirrors a libobs C enum, with conversions
/// to and from the underlying integer representation.
///
/// The macro expands to:
///
/// * The Rust enum itself, deriving `Debug`, `Clone`, `Copy`, `Eq`, and
///   `PartialEq`.
/// * Inherent `as_raw` and `from_raw` methods for explicit conversion to
///   and from the C enum's integer type. `from_raw` returns a
///   `Result<Self, NativeParsingError>`.
/// * `From<Self>` and `TryFrom<i32>` impls for use with the standard
///   conversion traits.
///
/// Used internally; plugin code typically only sees the resulting enum.
#[macro_export]
macro_rules! native_enum {
    ($(#[$($attrs_enum:tt)*])* $name:ident,$native_name:ident { $($(#[$($attrss:tt)*])* $rust:ident => $native:ident,)* }) => {
        paste::item! {
            $(#[$($attrs_enum)*])*
            #[derive(Debug, Clone, Copy, Eq, PartialEq)]
            pub enum $name {
                $(
                    $(#[$($attrss)*])*
                    $rust,
                )*
            }

            impl $name {
                pub fn as_raw(&self) -> $native_name {
                    match self {
                        $(Self::$rust => [<$native_name _ $native>],)*
                    }
                }

                #[allow(non_upper_case_globals)]
                pub fn from_raw(value: $native_name) -> Result<Self, $crate::native_enum::NativeParsingError> {
                    match value {
                        $([<$native_name _ $native>] => Ok(Self::$rust)),*,
                        _ => Err($crate::native_enum::NativeParsingError::new(stringify!($name), value as i64))
                    }
                }
            }

            #[allow(clippy::from_over_into)]
            impl Into<$native_name> for $name {
                fn into(self) -> $native_name {
                    self.as_raw()
                }
            }

            impl std::convert::TryFrom<$native_name> for $name {
                type Error = $crate::native_enum::NativeParsingError;
                fn try_from(value: $native_name) -> Result<Self, $crate::native_enum::NativeParsingError> {
                    Self::from_raw(value)
                }
            }
        }
    };
}
