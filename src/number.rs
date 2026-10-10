//! Number limits, and keys or literals as operands.

use crate::sealed::Sealed;
use crate::{Error, Operand, json};

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

use bigdecimal::BigDecimal;

/// Most digits a number may have. Stops requests like `1e-1000000 + 1`.
pub const MAX_DIGITS: u64 = 65_536;

/// Largest exponent, either sign. `1e999999` is one digit, so it's fine.
pub const MAX_SCALE: i64 = 1_000_000;

/// Rejects values outside [`MAX_DIGITS`] and [`MAX_SCALE`].
pub(crate) fn check(value: &BigDecimal) -> Result<(), Error> {
    let (_, scale) = value.as_bigint_and_scale();
    if value.digits() > MAX_DIGITS || scale.unsigned_abs() > MAX_SCALE.unsigned_abs() {
        return Err(Error::OutOfRange);
    }
    Ok(())
}

impl Sealed for str {}
impl Sealed for String {}
impl Sealed for BigDecimal {}

impl Operand for str {
    fn resolve(
        &self,
        lookup: impl FnOnce(&str) -> Result<BigDecimal, Error>,
    ) -> Result<Cow<'_, BigDecimal>, Error> {
        lookup(self).map(Cow::Owned)
    }

    fn write_json(&self, out: &mut Vec<u8>) {
        json::write_str(out, self);
    }
}

impl Operand for BigDecimal {
    fn resolve(
        &self,
        _: impl FnOnce(&str) -> Result<BigDecimal, Error>,
    ) -> Result<Cow<'_, BigDecimal>, Error> {
        Ok(Cow::Borrowed(self))
    }

    fn write_json(&self, out: &mut Vec<u8>) {
        // BigDecimal's `Display` output is valid JSON.
        json::write_display(out, self);
    }
}

impl Operand for String {
    fn resolve(
        &self,
        lookup: impl FnOnce(&str) -> Result<BigDecimal, Error>,
    ) -> Result<Cow<'_, BigDecimal>, Error> {
        self.as_str().resolve(lookup)
    }

    fn write_json(&self, out: &mut Vec<u8>) {
        self.as_str().write_json(out);
    }
}

// Integers are literals. Floats are left out on purpose: they would round.
macro_rules! integer_operands {
    ($($t:ty),*) => {$(
        impl Sealed for $t {}

        impl Operand for $t {
            fn resolve(
                &self,
                _: impl FnOnce(&str) -> Result<BigDecimal, Error>,
            ) -> Result<Cow<'_, BigDecimal>, Error> {
                Ok(Cow::Owned(BigDecimal::from(*self)))
            }

            fn write_json(&self, out: &mut Vec<u8>) {
                json::write_display(out, self);
            }
        }
    )*};
}

integer_operands!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);
