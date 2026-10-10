//! Arithmetic operations, one type each.

use crate::sealed::Sealed;
use crate::{Error, MAX_DIGITS, Op};

use core::fmt;

use bigdecimal::{BigDecimal, Zero};

/// `lhs + rhs`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Add;

/// `lhs - rhs`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Subtract;

/// `lhs * rhs`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Multiply;

/// `lhs / rhs`, rounded to 100 significant digits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Divide;

macro_rules! name_ops {
    ($($t:ty),*) => {$(
        impl Sealed for $t {}

        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(<$t as Op>::NAME)
            }
        }
    )*};
}

name_ops!(Add, Subtract, Multiply, Divide);

impl Op for Add {
    const NAME: &'static str = "add";

    fn eval(lhs: &BigDecimal, rhs: &BigDecimal) -> Result<BigDecimal, Error> {
        fits_aligned(lhs, rhs)?;
        Ok(lhs + rhs)
    }
}

impl Op for Subtract {
    const NAME: &'static str = "subtract";

    fn eval(lhs: &BigDecimal, rhs: &BigDecimal) -> Result<BigDecimal, Error> {
        fits_aligned(lhs, rhs)?;
        Ok(lhs - rhs)
    }
}

impl Op for Multiply {
    const NAME: &'static str = "multiply";

    fn eval(lhs: &BigDecimal, rhs: &BigDecimal) -> Result<BigDecimal, Error> {
        Ok(lhs * rhs)
    }
}

impl Op for Divide {
    const NAME: &'static str = "divide";

    fn eval(lhs: &BigDecimal, rhs: &BigDecimal) -> Result<BigDecimal, Error> {
        if rhs.is_zero() {
            return Err(Error::DivisionByZero);
        }
        Ok(lhs / rhs)
    }
}

/// Adding needs both numbers at the same scale. Check the size first.
fn fits_aligned(lhs: &BigDecimal, rhs: &BigDecimal) -> Result<(), Error> {
    if lhs.is_zero() || rhs.is_zero() {
        return Ok(());
    }
    let (ls, rs) = (lhs.as_bigint_and_scale().1, rhs.as_bigint_and_scale().1);
    let widest = if ls < rs { lhs.digits() } else { rhs.digits() };
    if widest.saturating_add(ls.abs_diff(rs)) > MAX_DIGITS {
        return Err(Error::OutOfRange);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> BigDecimal {
        s.parse().unwrap()
    }

    #[test]
    fn huge_alignment_is_refused_before_allocating() {
        let r = Add::apply(&dec("1e-1000000"), &dec("1"));
        assert!(matches!(r, Err(Error::OutOfRange)));
        let big = dec("1e999999");
        assert_eq!(Multiply::apply(&big, &dec("2")).unwrap(), dec("2e999999"));
        assert!(matches!(
            Multiply::apply(&big, &big),
            Err(Error::OutOfRange)
        ));
    }

    #[test]
    fn divide_by_zero() {
        assert!(matches!(
            Divide::apply(&dec("1"), &dec("0")),
            Err(Error::DivisionByZero)
        ));
    }
}
