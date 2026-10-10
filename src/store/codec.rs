//! How a decimal is stored: digits as little-endian bytes, then the scale.

use crate::Error;

use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::BigInt;

/// Bytes of the `i64` scale at the end of each stored value.
const SCALE_LEN: usize = size_of::<i64>();

/// Digits as little-endian bytes, then the scale as `i64`.
pub(super) fn encode(value: &BigDecimal) -> Vec<u8> {
    let (digits, scale) = value.as_bigint_and_scale();
    let mut bytes = digits.to_signed_bytes_le();
    bytes.extend_from_slice(&scale.to_le_bytes());
    bytes
}

pub(super) fn decode(bytes: &[u8]) -> Result<BigDecimal, Error> {
    // Zero still has one digit byte.
    if bytes.len() <= SCALE_LEN {
        return Err(Error::Corrupt);
    }
    let (digits, scale) = bytes.split_at(bytes.len() - SCALE_LEN);
    let scale = i64::from_le_bytes(scale.try_into().map_err(|_| Error::Corrupt)?);
    Ok(BigDecimal::new(BigInt::from_signed_bytes_le(digits), scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> BigDecimal {
        s.parse().unwrap()
    }

    #[test]
    fn encoding_round_trips() {
        for s in [
            "0",
            "-1",
            "6416930923733925522307001.29472615",
            "-0.000000000066731039356729",
            "1e-400",
            "-7e300",
        ] {
            let v = dec(s);
            assert_eq!(decode(&encode(&v)).unwrap(), v, "{s}");
        }
        assert!(matches!(decode(&[0; SCALE_LEN]), Err(Error::Corrupt)));
    }
}
