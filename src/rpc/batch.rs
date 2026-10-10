//! Batches, run one element at a time.

use super::envelope::Envelope;
use super::reply::{Fault, write_reply};
use crate::{Backend, V2};

use alloc::vec::Vec;
use core::fmt;

use serde::de::{Deserializer, IgnoredAny, SeqAccess, Visitor};
use serde_json::value::RawValue;

pub(super) fn run<B: Backend>(backend: &B, ns: &str, input: &[u8], out: &mut Vec<u8>) -> bool {
    // A broken batch must run nothing, so check the syntax first.
    if serde_json::from_slice::<IgnoredAny>(input).is_err() {
        write_reply::<V2, bool>(out, None, Err(Fault::PARSE));
        return true;
    }
    let start = out.len();
    out.push(b'[');
    let mut batch = Batch {
        backend,
        ns,
        out,
        seen: 0,
        written: 0,
    };
    let mut de = serde_json::Deserializer::from_slice(input);
    // Can't fail: the syntax was checked above.
    let _ = de.deserialize_seq(&mut batch);
    let (seen, written) = (batch.seen, batch.written);
    if written > 0 {
        out.push(b']');
        return true;
    }
    out.truncate(start);
    if seen == 0 {
        // An empty batch gets one error, not an array.
        write_reply::<V2, bool>(out, None, Err(Fault::INVALID_REQUEST));
        return true;
    }
    false
}

struct Batch<'s, B> {
    backend: &'s B,
    ns: &'s str,
    out: &'s mut Vec<u8>,
    seen: usize,
    written: usize,
}

impl<'de, B: Backend> Visitor<'de> for &mut Batch<'_, B> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a batch of requests")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(element) = seq.next_element::<&'de RawValue>()? {
            self.seen += 1;
            let mark = self.out.len();
            if self.written > 0 {
                self.out.push(b',');
            }
            // Non-objects like `1` get their own Invalid Request reply. Arrays
            // are checked here because serde would accept them as a struct.
            let envelope = match element.get().starts_with('{') {
                true => serde_json::from_str::<Envelope<'_>>(element.get()).ok(),
                false => None,
            };
            let wrote = match envelope {
                Some(envelope) => super::call(self.backend, self.ns, envelope, self.out),
                None => {
                    write_reply::<V2, bool>(self.out, None, Err(Fault::INVALID_REQUEST));
                    true
                }
            };
            if wrote {
                self.written += 1;
            } else {
                self.out.truncate(mark);
            }
        }
        Ok(())
    }
}
