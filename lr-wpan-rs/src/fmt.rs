#![macro_use]
#![allow(unused)]

use core::fmt::{Debug, Display, LowerHex};

#[cfg(all(feature = "defmt-03", feature = "log-04"))]
compile_error!("You may not enable both `defmt` and `log` features.");

#[collapse_debuginfo(yes)]
macro_rules! assert {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::assert!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::assert!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! assert_eq {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::assert_eq!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::assert_eq!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! assert_ne {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::assert_ne!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::assert_ne!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! debug_assert {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::debug_assert!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::debug_assert!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! debug_assert_eq {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::debug_assert_eq!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::debug_assert_eq!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! debug_assert_ne {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::debug_assert_ne!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::debug_assert_ne!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! todo {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::todo!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::todo!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! unreachable {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::unreachable!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::unreachable!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! panic {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt-03"))]
            ::core::panic!($($x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::panic!($($x)*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! trace {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::trace!($s $(, $x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::trace!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04", feature="defmt-03")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! debug {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::debug!($s $(, $x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::debug!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04", feature="defmt-03")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! info {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::info!($s $(, $x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::info!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04", feature="defmt-03")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! warn {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::warn!($s $(, $x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::warn!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04", feature="defmt-03")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[collapse_debuginfo(yes)]
macro_rules! error {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::error!($s $(, $x)*);
            #[cfg(feature = "defmt-03")]
            ::defmt::error!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04", feature="defmt-03")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[cfg(feature = "defmt-03")]
#[collapse_debuginfo(yes)]
macro_rules! unwrap {
    ($($x:tt)*) => {
        ::defmt::unwrap!($($x)*)
    };
}

#[cfg(not(feature = "defmt-03"))]
#[collapse_debuginfo(yes)]
macro_rules! unwrap {
    ($arg:expr) => {
        match $crate::fmt::Try::into_result($arg) {
            ::core::result::Result::Ok(t) => t,
            ::core::result::Result::Err(e) => {
                ::core::panic!("unwrap of `{}` failed: {:?}", ::core::stringify!($arg), e);
            }
        }
    };
    ($arg:expr, $($msg:expr),+ $(,)? ) => {
        match $crate::fmt::Try::into_result($arg) {
            ::core::result::Result::Ok(t) => t,
            ::core::result::Result::Err(e) => {
                ::core::panic!("unwrap of `{}` failed: {}: {:?}", ::core::stringify!($arg), ::core::format_args!($($msg,)*), e);
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NoneError;

pub trait Try {
    type Ok;
    type Error;
    fn into_result(self) -> Result<Self::Ok, Self::Error>;
}

impl<T> Try for Option<T> {
    type Ok = T;
    type Error = NoneError;

    #[inline]
    fn into_result(self) -> Result<T, NoneError> {
        self.ok_or(NoneError)
    }
}

impl<T, E> Try for Result<T, E> {
    type Ok = T;
    type Error = E;

    #[inline]
    fn into_result(self) -> Self {
        self
    }
}

pub(crate) struct Bytes<'a>(pub &'a [u8]);

impl Debug for Bytes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#02x?}", self.0)
    }
}

impl Display for Bytes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#02x?}", self.0)
    }
}

impl LowerHex for Bytes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#02x?}", self.0)
    }
}

#[cfg(feature = "defmt-03")]
impl<'a> defmt::Format for Bytes<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{:02x}", self.0)
    }
}
