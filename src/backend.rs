//! `Backend` for references and smart pointers, so a shared `Arc<Store>` works anywhere.

use crate::{Backend, BigDecimal, Error, Op, Operand};

use alloc::boxed::Box;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;

macro_rules! forward {
    ($($ptr:ty),*) => {$(
        impl<B: Backend> Backend for $ptr {
            fn create(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<(), Error> {
                (**self).create(ns, key, value)
            }

            fn read(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
                (**self).read(ns, key)
            }

            fn update(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<BigDecimal, Error> {
                (**self).update(ns, key, value)
            }

            fn delete(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
                (**self).delete(ns, key)
            }

            fn compute<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
                &self,
                ns: &str,
                op: O,
                lhs: &L,
                rhs: &R,
            ) -> Result<BigDecimal, Error> {
                (**self).compute(ns, op, lhs, rhs)
            }

            fn compute_into<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
                &self,
                ns: &str,
                op: O,
                lhs: &L,
                rhs: &R,
                into: &str,
            ) -> Result<BigDecimal, Error> {
                (**self).compute_into(ns, op, lhs, rhs, into)
            }
        }
    )*};
}

forward!(&B, Box<B>);
#[cfg(target_has_atomic = "ptr")]
forward!(Arc<B>);
