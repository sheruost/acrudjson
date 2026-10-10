//! One type per method, shared by the client and the server.
//!
//! The client writes these as requests. The server reads their field names,
//! by reflection, as each method's params.

use crate::op::Add;
use crate::reflect::field_names;
use crate::sealed::Sealed;
use crate::{Call, Op, Operand};

use alloc::vec::Vec;

use bigdecimal::BigDecimal;

/// `create(key, value)`
#[derive(Debug, Clone, Copy)]
pub struct Create<'a> {
    /// The key to create.
    pub key: &'a str,
    /// The value to store.
    pub value: &'a BigDecimal,
}

/// `read(key)`
#[derive(Debug, Clone, Copy)]
pub struct Read<'a> {
    /// The key to read.
    pub key: &'a str,
}

/// `update(key, value)`, which returns the old value.
#[derive(Debug, Clone, Copy)]
pub struct Update<'a> {
    /// The key to update.
    pub key: &'a str,
    /// The new value.
    pub value: &'a BigDecimal,
}

/// `delete(key)`, which returns the removed value.
#[derive(Debug, Clone, Copy)]
pub struct Delete<'a> {
    /// The key to remove.
    pub key: &'a str,
}

/// `lhs op rhs`. Operands are keys (`str`) or literals (`BigDecimal`).
#[derive(Debug, Clone, Copy)]
pub struct Compute<'a, O, L: ?Sized, R: ?Sized> {
    /// Left side.
    pub lhs: &'a L,
    /// Right side.
    pub rhs: &'a R,
    /// The operation, such as [`Multiply`](crate::op::Multiply).
    pub op: O,
}

/// `lhs op rhs`, saved under `into`.
#[derive(Debug, Clone, Copy)]
pub struct ComputeInto<'a, O, L: ?Sized, R: ?Sized> {
    /// Left side.
    pub lhs: &'a L,
    /// Right side.
    pub rhs: &'a R,
    /// Key to save the result under.
    pub into: &'a str,
    /// The operation, such as [`Multiply`](crate::op::Multiply).
    pub op: O,
}

impl Sealed for Create<'_> {}
impl Sealed for Read<'_> {}
impl Sealed for Update<'_> {}
impl Sealed for Delete<'_> {}
impl<O, L: ?Sized, R: ?Sized> Sealed for Compute<'_, O, L, R> {}
impl<O, L: ?Sized, R: ?Sized> Sealed for ComputeInto<'_, O, L, R> {}

impl Call for Create<'_> {
    const METHOD: &'static str = "create";

    fn write_params(&self, out: &mut Vec<u8>) {
        write_params(out, |out| {
            self.key.write_json(out);
            out.push(b',');
            self.value.write_json(out);
        });
    }
}

impl Call for Read<'_> {
    const METHOD: &'static str = "read";

    fn write_params(&self, out: &mut Vec<u8>) {
        write_params(out, |out| self.key.write_json(out));
    }
}

impl Call for Update<'_> {
    const METHOD: &'static str = "update";

    fn write_params(&self, out: &mut Vec<u8>) {
        write_params(out, |out| {
            self.key.write_json(out);
            out.push(b',');
            self.value.write_json(out);
        });
    }
}

impl Call for Delete<'_> {
    const METHOD: &'static str = "delete";

    fn write_params(&self, out: &mut Vec<u8>) {
        write_params(out, |out| self.key.write_json(out));
    }
}

impl<O: Op, L: Operand + ?Sized, R: Operand + ?Sized> Call for Compute<'_, O, L, R> {
    const METHOD: &'static str = O::NAME;

    fn write_params(&self, out: &mut Vec<u8>) {
        write_params(out, |out| {
            self.lhs.write_json(out);
            out.push(b',');
            self.rhs.write_json(out);
        });
    }
}

impl<O: Op, L: Operand + ?Sized, R: Operand + ?Sized> Call for ComputeInto<'_, O, L, R> {
    const METHOD: &'static str = O::NAME;

    fn write_params(&self, out: &mut Vec<u8>) {
        write_params(out, |out| {
            self.lhs.write_json(out);
            out.push(b',');
            self.rhs.write_json(out);
            out.push(b',');
            self.into.write_json(out);
        });
    }
}

/// Writes `[ ... ]` around what `items` writes.
fn write_params(out: &mut Vec<u8>, items: impl FnOnce(&mut Vec<u8>)) {
    out.push(b'[');
    items(out);
    out.push(b']');
}

// Param names, read from the types above.
pub(crate) const CREATE: [&str; 2] = field_names::<Create<'static>, 2>();
pub(crate) const READ: [&str; 1] = field_names::<Read<'static>, 1>();
pub(crate) const UPDATE: [&str; 2] = field_names::<Update<'static>, 2>();
pub(crate) const DELETE: [&str; 1] = field_names::<Delete<'static>, 1>();
const COMPUTE_INTO: [&str; 4] = field_names::<ComputeInto<'static, Add, str, str>, 4>();
// `op` is the last field and isn't a param.
pub(crate) const COMPUTE: [&str; 3] = [COMPUTE_INTO[0], COMPUTE_INTO[1], COMPUTE_INTO[2]];
