//! Reading params: slots, keys, numbers and operands.

use super::envelope::Text;
use super::reply::Fault;
use crate::number::check;
use crate::sealed::Sealed;
use crate::{Error, MAX_DIGITS, Operand};

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use bigdecimal::BigDecimal;
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::value::RawValue;

/// Reads up to `N` params, from an array or an object, into a fixed array.
pub(super) fn slots<'a, const N: usize>(
    params: Option<&'a RawValue>,
    names: &'static [&'static str; N],
) -> Result<[Option<&'a RawValue>; N], Fault> {
    let Some(params) = params else {
        return Ok([None; N]);
    };
    let mut de = serde_json::Deserializer::from_str(params.get());
    Slots(names)
        .deserialize(&mut de)
        .map_err(|_| Fault::invalid_params(names))
}

struct Slots<const N: usize>(&'static [&'static str; N]);

impl<'de, const N: usize> DeserializeSeed<'de> for Slots<N> {
    type Value = [Option<&'de RawValue>; N];

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de, const N: usize> Visitor<'de> for Slots<N> {
    type Value = [Option<&'de RawValue>; N];

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at most {N} params")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut slots = [None; N];
        for slot in &mut slots {
            match seq.next_element()? {
                Some(value) => *slot = Some(value),
                None => return Ok(slots),
            }
        }
        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(de::Error::invalid_length(N + 1, &self));
        }
        Ok(slots)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut slots = [None; N];
        while let Some(name) = map.next_key::<&'de str>()? {
            let Some(i) = self.0.iter().position(|n| *n == name) else {
                return Err(de::Error::unknown_field(name, &[]));
            };
            if slots[i].is_some() {
                return Err(de::Error::duplicate_field(self.0[i]));
            }
            slots[i] = Some(map.next_value()?);
        }
        Ok(slots)
    }
}

/// A key: a JSON string.
pub(super) fn text<'a>(
    raw: Option<&'a RawValue>,
    names: &'static [&'static str],
) -> Result<Cow<'a, str>, Fault> {
    let raw = raw.ok_or(Fault::invalid_params(names))?;
    match serde_json::from_str::<Text<'a>>(raw.get()) {
        Ok(Text(s)) => Ok(s),
        Err(_) => Err(Fault::invalid_params(names)),
    }
}

/// A value: parsed from its JSON text, never through `f64`.
pub(super) fn number(
    raw: Option<&RawValue>,
    names: &'static [&'static str],
) -> Result<BigDecimal, Fault> {
    let raw = raw.ok_or(Fault::invalid_params(names))?;
    let source = raw.get();
    let digits = match source.as_bytes().first() {
        Some(b'-' | b'0'..=b'9') => Cow::Borrowed(source),
        Some(b'"') => text(Some(raw), names)?,
        _ => return Err(Fault::invalid_params(names)),
    };
    // Check the length before parsing.
    if digits.len() as u64 > MAX_DIGITS + 32 {
        return Err(Error::OutOfRange.into());
    }
    let value = BigDecimal::from_str(&digits).map_err(|_| Fault::invalid_params(names))?;
    check(&value)?;
    Ok(value)
}

/// An operand from JSON, where the value's type decides key or literal.
pub(super) enum Arg<'a> {
    Key(Cow<'a, str>),
    Literal(BigDecimal),
}

impl Sealed for Arg<'_> {}

impl Operand for Arg<'_> {
    fn resolve(
        &self,
        lookup: impl FnOnce(&str) -> Result<BigDecimal, Error>,
    ) -> Result<Cow<'_, BigDecimal>, Error> {
        match self {
            Arg::Key(key) => key.as_ref().resolve(lookup),
            Arg::Literal(value) => value.resolve(lookup),
        }
    }

    fn write_json(&self, out: &mut Vec<u8>) {
        match self {
            Arg::Key(key) => key.as_ref().write_json(out),
            Arg::Literal(value) => value.write_json(out),
        }
    }
}

/// A string is a key, anything else must be a number.
pub(super) fn operand<'a>(
    raw: Option<&'a RawValue>,
    names: &'static [&'static str],
) -> Result<Arg<'a>, Fault> {
    match raw.map(|r| r.get().as_bytes().first()) {
        Some(Some(b'"')) => text(raw, names).map(Arg::Key),
        _ => number(raw, names).map(Arg::Literal),
    }
}
