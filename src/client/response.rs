//! Reading replies.

use alloc::borrow::Cow;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

/// The `error` member of a response.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcError<'a> {
    /// See [`code`](crate::code).
    pub code: i64,
    /// A short description.
    #[serde(borrow)]
    pub message: Cow<'a, str>,
    /// Extra detail, such as the expected params.
    #[serde(borrow, default, deserialize_with = "borrowed")]
    pub data: Option<Cow<'a, RawValue>>,
}

fn borrowed<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Cow<'de, RawValue>>, D::Error> {
    Option::<&'de RawValue>::deserialize(d).map(|raw| raw.map(Cow::Borrowed))
}

impl RpcError<'_> {
    /// Copies the error out of the reply bytes, so `?` can box it.
    pub fn into_owned(self) -> RpcError<'static> {
        RpcError {
            code: self.code,
            message: Cow::Owned(self.message.into_owned()),
            data: self.data.map(|data| Cow::Owned(data.into_owned())),
        }
    }
}

impl fmt::Display for RpcError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)?;
        if let Some(data) = &self.data {
            write!(f, ": {}", data.get())?;
        }
        Ok(())
    }
}

impl core::error::Error for RpcError<'_> {}

/// A reply, borrowed from the bytes it was parsed from.
#[derive(Debug, Clone, Deserialize)]
pub struct Response<'a> {
    #[serde(borrow, default)]
    result: Option<&'a RawValue>,
    #[serde(borrow, default)]
    error: Option<RpcError<'a>>,
    /// The request id, as raw JSON.
    #[serde(borrow)]
    pub id: &'a RawValue,
}

impl<'a> TryFrom<&'a [u8]> for Response<'a> {
    type Error = serde_json::Error;

    /// Parses a single reply.
    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

impl<'a> Response<'a> {
    /// Parses a single reply or a batch of replies.
    pub fn parse_many(bytes: &'a [u8]) -> Result<Vec<Self>, serde_json::Error> {
        if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'[') {
            serde_json::from_slice(bytes)
        } else {
            Self::try_from(bytes).map(|r| vec![r])
        }
    }

    /// The id as a number, if it is one.
    pub fn id(&self) -> Option<u64> {
        self.id.get().parse().ok()
    }

    /// The raw `result`, or the server's error.
    ///
    /// The error borrows from the reply bytes. Call [`RpcError::into_owned`]
    /// to `?` it.
    pub fn result(&self) -> Result<&'a RawValue, &RpcError<'a>> {
        match (&self.error, self.result) {
            (Some(e), _) => Err(e),
            (None, result) => Ok(result.unwrap_or(RawValue::NULL)),
        }
    }

    /// The `result` as a decimal. `None` if it isn't a number.
    pub fn number(&self) -> Result<Option<BigDecimal>, &RpcError<'a>> {
        Ok(BigDecimal::from_str(self.result()?.get()).ok())
    }
}
