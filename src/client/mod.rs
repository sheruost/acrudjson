//! Build requests and read replies.
//!
//! Each method is its own type implementing [`Call`]. The version is a type
//! too: [`V1`](crate::V1) or [`V2`](crate::V2).
//!
//! ```
//! use acrudjson::client::{self, Create, Response};
//! use acrudjson::{Backend, Store, V2};
//!
//! let store = Store::in_memory()?;
//! let g = "6.6743e-11".parse()?;
//!
//! let mut request = Vec::new();
//! client::request::<V2, _>(&mut request, 1, &Create { key: "G", value: &g });
//!
//! let mut bytes = Vec::new();
//! store.handle("default", &request, &mut bytes);
//! let response = Response::try_from(&bytes[..])?;
//! assert!(matches!(response.result(), Ok(raw) if raw.get() == "true"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod batch;
mod response;

pub use crate::call::{Compute, ComputeInto, Create, Delete, Read, Update};
pub use batch::Batch;
pub use response::{Response, RpcError};

use crate::{Call, Operand, Version, json};

use alloc::vec::Vec;

/// Appends a request with the given `id` to `out`.
pub fn request<V: Version, C: Call>(out: &mut Vec<u8>, id: u64, call: &C) {
    write_call::<V, C>(out, call);
    out.extend_from_slice(br#","id":"#);
    json::write_display(out, id);
    out.push(b'}');
}

/// Appends a notification (no reply expected) to `out`.
pub fn notification<V: Version, C: Call>(out: &mut Vec<u8>, call: &C) {
    write_call::<V, C>(out, call);
    out.extend_from_slice(V::NOTIFICATION_ID);
    out.push(b'}');
}

fn write_call<V: Version, C: Call>(out: &mut Vec<u8>, call: &C) {
    out.extend_from_slice(V::OPEN);
    out.extend_from_slice(br#""method":"#);
    C::METHOD.write_json(out);
    out.extend_from_slice(br#","params":"#);
    call.write_params(out);
}
