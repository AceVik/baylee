//! Base64 (RFC 4648 §4, padded), for the one binary thing a report carries.
//!
//! Twenty lines rather than a dependency: the screenshot is the only caller,
//! it only ever encodes, and the standard's own test vectors pin it.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` in base64, padded with `=`.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(n >> shift & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `text` decoded, for a test reading back what a report carries. Panics on
/// anything [`encode`] would not have written.
#[cfg(test)]
pub(crate) fn decode_for_tests(text: &str) -> Vec<u8> {
    let mut bits = 0u32;
    let mut held = 0;
    let mut out = Vec::new();
    for byte in text.bytes().filter(|b| *b != b'=') {
        let value = ALPHABET
            .iter()
            .position(|a| *a == byte)
            .expect("a base64 character");
        bits = bits << 6 | u32::try_from(value).expect("under 64");
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push(u8::try_from(bits >> held & 0xff).expect("a byte"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decode_for_tests, encode};

    /// RFC 4648 §10.
    #[test]
    fn the_standard_s_test_vectors() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), coded, "{plain:?}");
            assert_eq!(decode_for_tests(coded), plain.as_bytes(), "{coded:?}");
        }
        assert_eq!(encode(&[0xfb, 0xff]), "+/8=");
    }
}
