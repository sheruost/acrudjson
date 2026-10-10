use crate::Version;
use crate::sealed::Sealed;

use serde_json::value::RawValue;

/// JSON-RPC 1.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct V1;

/// JSON-RPC 2.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct V2;

impl Sealed for V1 {}
impl Sealed for V2 {}

impl Version for V1 {
    const OPEN: &'static [u8] = b"{";
    const NOTIFICATION_ID: &'static [u8] = br#","id":null"#;
    const RESULT_END: &'static [u8] = br#","error":null"#;
    const ERROR_START: &'static [u8] = br#""result":null,"error":"#;

    fn reply_id(id: Option<&RawValue>) -> Option<Option<&RawValue>> {
        match id {
            Some(id) if id.get() == "null" => Some(None),
            Some(id) => Some(Some(id)),
            None => None,
        }
    }
}

impl Version for V2 {
    const OPEN: &'static [u8] = br#"{"jsonrpc":"2.0","#;
    const NOTIFICATION_ID: &'static [u8] = b"";
    const RESULT_END: &'static [u8] = b"";
    const ERROR_START: &'static [u8] = br#""error":"#;

    fn reply_id(id: Option<&RawValue>) -> Option<Option<&RawValue>> {
        match id {
            None => Some(None),
            Some(id) if valid_id(id) => Some(Some(id)),
            Some(_) => None,
        }
    }
}

/// A 2.0 id must be a string, a number or null.
pub(crate) fn valid_id(id: &RawValue) -> bool {
    matches!(
        id.get().as_bytes().first(),
        Some(b'"' | b'-' | b'0'..=b'9' | b'n')
    )
}
