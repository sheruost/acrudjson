//! JSON-RPC 2.0 batches.

use super::{notification, request};
use crate::{Call, V2};

use alloc::vec::Vec;

/// A JSON-RPC 2.0 batch being written.
///
/// A batch starts from its first call, so it can never be empty.
///
/// ```
/// use acrudjson::client::{Batch, Delete, Read};
///
/// let mut out = Vec::new();
/// Batch::request(&mut out, 1, &Read { key: "a" })
///     .and_notification(&Delete { key: "b" })
///     .finish();
/// assert!(out.starts_with(b"[{") && out.ends_with(b"}]"));
/// ```
#[derive(Debug)]
#[must_use = "call `finish` to close the batch"]
pub struct Batch<'o> {
    out: &'o mut Vec<u8>,
}

impl<'o> Batch<'o> {
    /// Starts a batch at the end of `out` with a request.
    pub fn request<C: Call>(out: &'o mut Vec<u8>, id: u64, call: &C) -> Self {
        out.push(b'[');
        request::<V2, C>(out, id, call);
        Batch { out }
    }

    /// Starts a batch at the end of `out` with a notification.
    pub fn notification<C: Call>(out: &'o mut Vec<u8>, call: &C) -> Self {
        out.push(b'[');
        notification::<V2, C>(out, call);
        Batch { out }
    }

    /// Adds a request.
    pub fn and_request<C: Call>(self, id: u64, call: &C) -> Self {
        self.out.push(b',');
        request::<V2, C>(self.out, id, call);
        self
    }

    /// Adds a notification.
    pub fn and_notification<C: Call>(self, call: &C) -> Self {
        self.out.push(b',');
        notification::<V2, C>(self.out, call);
        self
    }

    /// Closes the batch.
    pub fn finish(self) {
        self.out.push(b']');
    }
}
