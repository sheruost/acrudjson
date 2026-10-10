use alloc::boxed::Box;
use core::fmt;

/// Errors from a [`Backend`](crate::Backend).
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The key does not exist.
    KeyNotFound,
    /// The key already exists.
    KeyExists,
    /// Division by zero.
    DivisionByZero,
    /// A number is over [`MAX_DIGITS`](crate::MAX_DIGITS) or [`MAX_SCALE`](crate::MAX_SCALE).
    OutOfRange,
    /// A stored value can't be decoded.
    Corrupt,
    /// The storage failed.
    Storage(Box<dyn core::error::Error + Send + Sync>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::KeyNotFound => f.write_str("key not found"),
            Error::KeyExists => f.write_str("key already exists"),
            Error::DivisionByZero => f.write_str("division by zero"),
            Error::OutOfRange => f.write_str("number out of range"),
            Error::Corrupt => f.write_str("stored value is corrupt"),
            Error::Storage(e) => write!(f, "storage error: {e}"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Storage(e) => Some(&**e),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
macro_rules! from_redb {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for Error {
            #[cold]
            fn from(e: $t) -> Self {
                Error::Storage(Box::new(redb::Error::from(e)))
            }
        }
    )*};
}

#[cfg(feature = "std")]
from_redb!(
    redb::Error,
    redb::DatabaseError,
    redb::TransactionError,
    redb::TableError,
    redb::StorageError,
    redb::CommitError,
);

#[cfg(feature = "std")]
impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        use std::io::ErrorKind;
        let kind = match e {
            Error::KeyNotFound => ErrorKind::NotFound,
            Error::KeyExists => ErrorKind::AlreadyExists,
            Error::DivisionByZero | Error::OutOfRange => ErrorKind::InvalidInput,
            Error::Corrupt => ErrorKind::InvalidData,
            Error::Storage(_) => ErrorKind::Other,
        };
        std::io::Error::new(kind, e)
    }
}
