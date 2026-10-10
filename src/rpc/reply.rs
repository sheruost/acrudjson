//! Error codes and reply writing.

use crate::{Error, Operand, Version, json};

use alloc::vec::Vec;

use bigdecimal::BigDecimal;
use serde_json::value::RawValue;

/// Values of `error.code`. -32700 to -32600 come from the JSON-RPC spec.
pub mod code {
    /// The message is not valid JSON.
    pub const PARSE_ERROR: i32 = -32700;
    /// The JSON is not a valid request object.
    pub const INVALID_REQUEST: i32 = -32600;
    /// The method does not exist.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// The params are wrong.
    pub const INVALID_PARAMS: i32 = -32602;
    /// The storage failed.
    pub const INTERNAL_ERROR: i32 = -32603;
    /// See [`Error::KeyNotFound`](crate::Error::KeyNotFound).
    pub const KEY_NOT_FOUND: i32 = -32001;
    /// See [`Error::KeyExists`](crate::Error::KeyExists).
    pub const KEY_EXISTS: i32 = -32002;
    /// See [`Error::DivisionByZero`](crate::Error::DivisionByZero).
    pub const DIVISION_BY_ZERO: i32 = -32003;
    /// See [`Error::OutOfRange`](crate::Error::OutOfRange).
    pub const OUT_OF_RANGE: i32 = -32004;
}

/// An error reply. All `'static`, so it never allocates.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fault {
    pub(super) code: i32,
    message: &'static str,
    /// The expected param names.
    data: Option<&'static [&'static str]>,
}

impl Fault {
    const fn new(code: i32, message: &'static str) -> Self {
        Fault {
            code,
            message,
            data: None,
        }
    }

    pub(super) const PARSE: Fault = Fault::new(code::PARSE_ERROR, "Parse error");
    pub(super) const INVALID_REQUEST: Fault = Fault::new(code::INVALID_REQUEST, "Invalid Request");
    pub(super) const METHOD_NOT_FOUND: Fault =
        Fault::new(code::METHOD_NOT_FOUND, "Method not found");

    pub(super) const fn invalid_params(names: &'static [&'static str]) -> Self {
        Fault {
            code: code::INVALID_PARAMS,
            message: "Invalid params",
            data: Some(names),
        }
    }
}

impl From<Error> for Fault {
    #[cold]
    fn from(e: Error) -> Self {
        match e {
            Error::KeyNotFound => Fault::new(code::KEY_NOT_FOUND, "Key not found"),
            Error::KeyExists => Fault::new(code::KEY_EXISTS, "Key already exists"),
            Error::DivisionByZero => Fault::new(code::DIVISION_BY_ZERO, "Division by zero"),
            Error::OutOfRange => Fault::new(code::OUT_OF_RANGE, "Number out of range"),
            // Don't send storage details; they can contain file paths.
            Error::Corrupt | Error::Storage(_) => {
                Fault::new(code::INTERNAL_ERROR, "Internal error")
            }
        }
    }
}

/// A result value that can be written as JSON.
pub(super) trait Reply {
    fn write_result(&self, out: &mut Vec<u8>);
}

impl Reply for bool {
    fn write_result(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(if *self { b"true" } else { b"false" });
    }
}

impl Reply for BigDecimal {
    fn write_result(&self, out: &mut Vec<u8>) {
        Operand::write_json(self, out);
    }
}

/// Writes the reply, unless the request was a notification.
pub(super) fn finish<V: Version, T: Reply>(
    out: &mut Vec<u8>,
    id: Option<&RawValue>,
    result: Result<T, Fault>,
) -> bool {
    match (id, result) {
        (Some(id), result) => write_reply::<V, T>(out, Some(id), result),
        // A malformed request is answered even without an id.
        (None, Err(fault)) if fault.code == code::INVALID_REQUEST => {
            write_reply::<V, T>(out, None, Err(fault))
        }
        (None, _) => return false,
    }
    true
}

/// Writes a reply. `id: None` becomes `null`.
pub(super) fn write_reply<V: Version, T: Reply>(
    out: &mut Vec<u8>,
    id: Option<&RawValue>,
    result: Result<T, Fault>,
) {
    out.extend_from_slice(V::OPEN);
    match result {
        Ok(value) => {
            out.extend_from_slice(br#""result":"#);
            value.write_result(out);
            out.extend_from_slice(V::RESULT_END);
        }
        Err(fault) => {
            out.extend_from_slice(V::ERROR_START);
            out.extend_from_slice(br#"{"code":"#);
            json::write_display(out, fault.code);
            out.extend_from_slice(br#","message":"#);
            json::write_str(out, fault.message);
            if let Some(names) = fault.data {
                out.extend_from_slice(br#","data":["#);
                for (i, name) in names.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    json::write_str(out, name);
                }
                out.push(b']');
            }
            out.push(b'}');
        }
    }
    out.extend_from_slice(br#","id":"#);
    out.extend_from_slice(id.map_or("null", RawValue::get).as_bytes());
    out.push(b'}');
}
