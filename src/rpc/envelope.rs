//! The outer shape of a request.

use super::reply::Fault;

use alloc::borrow::Cow;

use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

/// A request's members as raw JSON.
///
/// `None` means missing, while `null` is `Some`. That's how 2.0 tells a
/// notification from a request with `"id": null`.
#[derive(Deserialize)]
pub(super) struct Envelope<'a> {
    #[serde(borrow, default, deserialize_with = "present")]
    pub(super) jsonrpc: Option<&'a RawValue>,
    #[serde(borrow, default, deserialize_with = "present")]
    pub(super) method: Option<&'a RawValue>,
    #[serde(borrow, default, deserialize_with = "present")]
    pub(super) params: Option<&'a RawValue>,
    #[serde(borrow, default, deserialize_with = "present")]
    pub(super) id: Option<&'a RawValue>,
}

fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<&'de RawValue>, D::Error> {
    <&RawValue>::deserialize(d).map(Some)
}

/// A JSON string. Borrowed unless it has escapes.
#[derive(Deserialize)]
pub(super) struct Text<'a>(#[serde(borrow)] pub(super) Cow<'a, str>);

impl<'a> Envelope<'a> {
    /// The method name, after checking that params, if any, are an array or object.
    pub(super) fn method_name(&self) -> Result<Cow<'a, str>, Fault> {
        let method = self.method.ok_or(Fault::INVALID_REQUEST)?;
        let Ok(Text(name)) = serde_json::from_str(method.get()) else {
            return Err(Fault::INVALID_REQUEST);
        };
        if let Some(p) = self.params
            && !matches!(p.get().as_bytes().first(), Some(b'[' | b'{'))
        {
            return Err(Fault::INVALID_REQUEST);
        }
        Ok(name)
    }
}
