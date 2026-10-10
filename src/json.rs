//! Small JSON writers that don't need `std::io`.

use alloc::vec::Vec;
use core::fmt::{self, Write as _};

/// Writes `s` as a JSON string.
pub(crate) fn write_str(out: &mut Vec<u8>, s: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = s.as_bytes();
    out.push(b'"');
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let escaped: &[u8] = match b {
            b'"' => br#"\""#,
            b'\\' => br"\\",
            b'\n' => br"\n",
            b'\r' => br"\r",
            b'\t' => br"\t",
            0x00..=0x1f => b"",
            _ => continue,
        };
        out.extend_from_slice(&bytes[start..i]);
        start = i + 1;
        if escaped.is_empty() {
            let (hi, lo) = (HEX[usize::from(b >> 4)], HEX[usize::from(b & 0xf)]);
            out.extend_from_slice(&[b'\\', b'u', b'0', b'0', hi, lo]);
        } else {
            out.extend_from_slice(escaped);
        }
    }
    out.extend_from_slice(&bytes[start..]);
    out.push(b'"');
}

/// Writes `value` using its `Display` output.
pub(crate) fn write_display(out: &mut Vec<u8>, value: impl fmt::Display) {
    struct Bytes<'a>(&'a mut Vec<u8>);

    impl fmt::Write for Bytes<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.0.extend_from_slice(s.as_bytes());
            Ok(())
        }
    }

    // Can't fail: writing to a Vec never errors.
    let _ = write!(Bytes(out), "{value}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_round_trip_through_serde_json() {
        for s in [
            "",
            "plain",
            "quote \" and \\",
            "line\nbreak\ttab\r",
            "\u{1}\u{1f}",
            "k\u{e9}y \u{1f600}",
        ] {
            let mut out = Vec::new();
            write_str(&mut out, s);
            assert_eq!(
                serde_json::from_slice::<alloc::string::String>(&out).unwrap(),
                s
            );
        }
    }
}
