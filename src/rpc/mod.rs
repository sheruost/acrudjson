//! The server side of the protocol: request bytes in, reply bytes out.
//!
//! Works over any [`Backend`]. Call it through [`Backend::handle`].

mod batch;
mod envelope;
mod params;
mod reply;

pub use reply::code;

use envelope::Envelope;
use params::{number, operand, slots, text};
use reply::{Fault, finish, write_reply};

use crate::call::{COMPUTE, CREATE, DELETE, READ, UPDATE};
use crate::op::{Add, Divide, Multiply, Subtract};
use crate::version::valid_id;
use crate::{Backend, Op, V1, V2, Version};

use alloc::vec::Vec;

use bigdecimal::BigDecimal;
use serde_json::value::RawValue;

pub(crate) fn handle<B: Backend>(backend: &B, ns: &str, input: &[u8], out: &mut Vec<u8>) -> bool {
    if input.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'[') {
        return batch::run(backend, ns, input, out);
    }
    match serde_json::from_slice::<Envelope<'_>>(input) {
        Ok(envelope) => call(backend, ns, envelope, out),
        Err(e) => {
            let fault = if e.is_data() {
                Fault::INVALID_REQUEST
            } else {
                Fault::PARSE
            };
            write_reply::<V2, bool>(out, None, Err(fault));
            true
        }
    }
}

/// Picks the version, then runs the request with it.
fn call<B: Backend>(backend: &B, ns: &str, env: Envelope<'_>, out: &mut Vec<u8>) -> bool {
    match env.jsonrpc {
        None => call_as::<V1, B>(backend, ns, env, out),
        Some(v) if v.get() == r#""2.0""# => call_as::<V2, B>(backend, ns, env, out),
        Some(_) => {
            let id = env.id.filter(|id| valid_id(id));
            write_reply::<V2, bool>(out, id, Err(Fault::INVALID_REQUEST));
            true
        }
    }
}

/// Runs one request. Writes a reply unless it's a notification.
fn call_as<V: Version, B: Backend>(
    backend: &B,
    ns: &str,
    env: Envelope<'_>,
    out: &mut Vec<u8>,
) -> bool {
    let Some(id) = V::reply_id(env.id) else {
        write_reply::<V, bool>(out, None, Err(Fault::INVALID_REQUEST));
        return true;
    };
    let name = match env.method_name() {
        Ok(name) => name,
        Err(fault) => return finish::<V, bool>(out, id, Err(fault)),
    };
    let params = env.params;
    match &*name {
        "create" => finish::<V, _>(out, id, create(backend, ns, params)),
        "read" => finish::<V, _>(out, id, read(backend, ns, params)),
        "update" => finish::<V, _>(out, id, update(backend, ns, params)),
        "delete" => finish::<V, _>(out, id, delete(backend, ns, params)),
        Add::NAME => finish::<V, _>(out, id, compute(backend, ns, Add, params)),
        Subtract::NAME => finish::<V, _>(out, id, compute(backend, ns, Subtract, params)),
        Multiply::NAME => finish::<V, _>(out, id, compute(backend, ns, Multiply, params)),
        Divide::NAME => finish::<V, _>(out, id, compute(backend, ns, Divide, params)),
        _ => finish::<V, bool>(out, id, Err(Fault::METHOD_NOT_FOUND)),
    }
}

fn create(b: &impl Backend, ns: &str, params: Option<&RawValue>) -> Result<bool, Fault> {
    let [key, value] = slots(params, &CREATE)?;
    b.create(ns, &text(key, &CREATE)?, &number(value, &CREATE)?)?;
    Ok(true)
}

fn read(b: &impl Backend, ns: &str, params: Option<&RawValue>) -> Result<BigDecimal, Fault> {
    let [key] = slots(params, &READ)?;
    Ok(b.read(ns, &text(key, &READ)?)?)
}

fn update(b: &impl Backend, ns: &str, params: Option<&RawValue>) -> Result<BigDecimal, Fault> {
    let [key, value] = slots(params, &UPDATE)?;
    Ok(b.update(ns, &text(key, &UPDATE)?, &number(value, &UPDATE)?)?)
}

fn delete(b: &impl Backend, ns: &str, params: Option<&RawValue>) -> Result<BigDecimal, Fault> {
    let [key] = slots(params, &DELETE)?;
    Ok(b.delete(ns, &text(key, &DELETE)?)?)
}

fn compute<O: Op>(
    b: &impl Backend,
    ns: &str,
    op: O,
    params: Option<&RawValue>,
) -> Result<BigDecimal, Fault> {
    let [lhs, rhs, into] = slots(params, &COMPUTE)?;
    let (lhs, rhs) = (operand(lhs, &COMPUTE)?, operand(rhs, &COMPUTE)?);
    match into.filter(|raw| raw.get() != "null") {
        Some(into) => Ok(b.compute_into(ns, op, &lhs, &rhs, &text(Some(into), &COMPUTE)?)?),
        None => Ok(b.compute(ns, op, &lhs, &rhs)?),
    }
}
