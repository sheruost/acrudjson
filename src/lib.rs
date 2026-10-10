//! Store big decimals and work on them over [JSON-RPC 1.0] or [JSON-RPC 2.0].
//!
//! The protocol side is `no_std` (it needs `alloc`). Storage needs `std`.
//!
//! | module | needs | what |
//! |---|---|---|
//! | [`rpc`] | `alloc` | turns request bytes into reply bytes over any [`Backend`] |
//! | [`client`] | `alloc` | builds requests, reads replies |
//! | [`op`] | `alloc` | `Add`, `Subtract`, `Multiply`, `Divide` |
//! | `Store` | `std` | a [`Backend`] on [redb] |
//! | `server` | `tokio` | UDP and TCP servers |
//!
//! Operations and protocol versions are types, not enum values:
//!
//! ```
//! # #[cfg(feature = "std")] {
//! use acrudjson::op::Multiply;
//! use acrudjson::{Backend, BigDecimal, Store};
//!
//! let store = Store::in_memory().unwrap();
//! store.create("lab", "G", &"6.6743e-11".parse().unwrap()).unwrap();
//! let doubled = store.compute("lab", Multiply, "G", &BigDecimal::from(2)).unwrap();
//! assert_eq!(doubled, "1.33486e-10".parse::<BigDecimal>().unwrap());
//! # }
//! ```
//!
//! | method | params | result |
//! |---|---|---|
//! | `create` | `key`, `value` | `true` |
//! | `read` | `key` | the value |
//! | `update` | `key`, `value` | the previous value |
//! | `delete` | `key` | the removed value |
//! | `add`, `subtract`, `multiply`, `divide` | `lhs`, `rhs`, optional `into` | the result |
//!
//! Params can be an array or an object. Keys are strings, values are numbers.
//! In arithmetic a string is a key and a number is a literal. `into` saves
//! the result under that key.
//!
//! [JSON-RPC 1.0]: https://www.jsonrpc.org/specification_v1
//! [JSON-RPC 2.0]: https://www.jsonrpc.org/specification
//! [redb]: https://docs.rs/redb

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(type_info)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod backend;
mod call;
pub mod client;
mod error;
mod json;
mod number;
pub mod op;
mod reflect;
pub mod rpc;
#[cfg(feature = "tokio")]
pub mod server;
#[cfg(feature = "std")]
mod store;
mod version;

pub use bigdecimal::BigDecimal;
pub use error::Error;
pub use number::{MAX_DIGITS, MAX_SCALE};
pub use rpc::code;
#[cfg(feature = "std")]
pub use store::Store;
pub use version::{V1, V2};

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

use serde_json::value::RawValue;

/// Keeps the crate's traits closed to outside implementations.
mod sealed {
    pub trait Sealed {}

    /// Lets the crate call `Op::eval`; other crates can't name or build it.
    #[derive(Debug, Clone, Copy)]
    pub struct Token;
}

/// Where values live: `Store` with `std`, or your own storage.
///
/// Each method works on one namespace `ns`, such as a user id.
pub trait Backend {
    /// Stores `value` under a new `key`.
    fn create(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<(), Error>;

    /// Returns the value stored under `key`.
    fn read(&self, ns: &str, key: &str) -> Result<BigDecimal, Error>;

    /// Replaces the value of `key` and returns the old one.
    fn update(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<BigDecimal, Error>;

    /// Removes `key` and returns the value it held.
    fn delete(&self, ns: &str, key: &str) -> Result<BigDecimal, Error>;

    /// Computes `lhs op rhs`.
    fn compute<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
        &self,
        ns: &str,
        op: O,
        lhs: &L,
        rhs: &R,
    ) -> Result<BigDecimal, Error>;

    /// Computes `lhs op rhs` and saves the result under `into`, atomically.
    fn compute_into<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
        &self,
        ns: &str,
        op: O,
        lhs: &L,
        rhs: &R,
        into: &str,
    ) -> Result<BigDecimal, Error>;

    /// Handles one JSON-RPC message and appends the reply to `out`.
    ///
    /// Returns `false` if there is no reply (only notifications). `out` is
    /// not cleared first.
    fn handle(&self, ns: &str, input: &[u8], out: &mut Vec<u8>) -> bool
    where
        Self: Sized,
    {
        rpc::handle(self, ns, input, out)
    }
}

/// An arithmetic operation: [`Add`](op::Add), [`Subtract`](op::Subtract),
/// [`Multiply`](op::Multiply) or [`Divide`](op::Divide).
///
/// `Display` prints the JSON-RPC method name.
///
/// Only [`Op::apply`] runs an operation from outside the crate, so the
/// [`MAX_DIGITS`] and [`MAX_SCALE`] checks can't be skipped:
///
/// ```compile_fail,E0061
/// use acrudjson::op::Multiply;
/// use acrudjson::{BigDecimal, Op};
///
/// let big = BigDecimal::from(1);
/// let _ = Multiply::eval(&big, &big);
/// ```
pub trait Op: sealed::Sealed + Copy + Default + fmt::Display {
    /// The JSON-RPC method name.
    const NAME: &'static str;

    #[doc(hidden)]
    fn eval(lhs: &BigDecimal, rhs: &BigDecimal, _: sealed::Token) -> Result<BigDecimal, Error>;

    /// Computes `lhs op rhs`, refusing numbers over [`MAX_DIGITS`] or [`MAX_SCALE`].
    fn apply(lhs: &BigDecimal, rhs: &BigDecimal) -> Result<BigDecimal, Error> {
        number::check(lhs)?;
        number::check(rhs)?;
        let result = Self::eval(lhs, rhs, sealed::Token)?;
        number::check(&result)?;
        Ok(result)
    }
}

/// One side of an [`Op`]: a key (`str`, `String`) or a literal (`BigDecimal`,
/// any integer). Floats are left out: they would round.
pub trait Operand: sealed::Sealed {
    /// Returns the literal, or looks the key up with `lookup`.
    fn resolve(
        &self,
        lookup: impl FnOnce(&str) -> Result<BigDecimal, Error>,
    ) -> Result<Cow<'_, BigDecimal>, Error>;

    /// Writes the operand as a JSON-RPC param.
    #[doc(hidden)]
    fn write_json(&self, out: &mut Vec<u8>);
}

/// A JSON-RPC version: [`V1`] or [`V2`].
///
/// Requests and replies differ only in a few fixed bytes, so a version is
/// just a set of constants.
pub trait Version: sealed::Sealed + Copy + Default {
    #[doc(hidden)]
    const OPEN: &'static [u8];
    #[doc(hidden)]
    const NOTIFICATION_ID: &'static [u8];
    #[doc(hidden)]
    const RESULT_END: &'static [u8];
    #[doc(hidden)]
    const ERROR_START: &'static [u8];

    /// The id to reply with: `Some(None)` for a notification, `None` if the
    /// id is not allowed.
    #[doc(hidden)]
    fn reply_id(id: Option<&RawValue>) -> Option<Option<&RawValue>>;
}

/// A method call: one type per method, in [`client`].
pub trait Call: sealed::Sealed {
    /// The JSON-RPC method name.
    const METHOD: &'static str;

    #[doc(hidden)]
    fn write_params(&self, out: &mut Vec<u8>);
}
