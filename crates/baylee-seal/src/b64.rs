//! Base64 (RFC 4648), in the two spellings this project writes.
//!
//! - [`url`]: §5, the URL-safe alphabet without padding. Every binary value
//!   on the social wire and in chat records is written this way.
//! - [`std_padded`]: §4, the standard alphabet padded with `=`. A bug
//!   report's screenshot and game record are written this way, and their
//!   bytes must not change.
//!
//! The two are separate modules so that nobody mixes them by accident: a
//! decoder accepts only what its own encoder would have written (its own
//! alphabet, its own padding rule, and zero bits after the last byte), so a
//! value in the wrong spelling is refused rather than read differently.
//!
//! Moved here from `baylee-client-core`'s `bugreport/base64.rs`, so that the
//! gateway and the client share one implementation.

use crate::Error;

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The standard alphabet, padded with `=` (RFC 4648 §4).
pub mod std_padded {
    use super::{Error, STANDARD, decode_with, encode_with};

    /// `bytes` in base64, padded with `=`.
    #[must_use]
    pub fn encode(bytes: &[u8]) -> String {
        encode_with(STANDARD, bytes, true)
    }

    /// `text` decoded.
    ///
    /// # Errors
    ///
    /// [`Error::Encoding`] for anything [`encode`] would not have written: a
    /// length that isn't a multiple of four, a character outside the
    /// alphabet, padding anywhere but the end, or set bits after the last
    /// byte.
    pub fn decode(text: &str) -> Result<Vec<u8>, Error> {
        decode_with(STANDARD, text, true)
    }
}

/// The URL-safe alphabet without padding (RFC 4648 §5, §3.2).
pub mod url {
    use super::{Error, URL_SAFE, decode_with, encode_with};

    /// `bytes` in URL-safe base64, without padding.
    #[must_use]
    pub fn encode(bytes: &[u8]) -> String {
        encode_with(URL_SAFE, bytes, false)
    }

    /// `text` decoded.
    ///
    /// # Errors
    ///
    /// [`Error::Encoding`] for anything [`encode`] would not have written: a
    /// `=`, a character outside the alphabet, a length that leaves one
    /// character over, or set bits after the last byte.
    pub fn decode(text: &str) -> Result<Vec<u8>, Error> {
        decode_with(URL_SAFE, text, false)
    }

    /// `text` decoded into exactly `N` bytes, for a key, nonce or signature.
    ///
    /// # Errors
    ///
    /// [`Error::Encoding`] as for [`decode`], and [`Error::Length`] when it
    /// decodes to any other number of bytes.
    pub fn decode_array<const N: usize>(text: &str) -> Result<[u8; N], Error> {
        decode(text)?.try_into().map_err(|_| Error::Length)
    }
}

fn encode_with(alphabet: &[u8; 64], bytes: &[u8], pad: bool) -> String {
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
                out.push(char::from(alphabet[(n >> shift & 63) as usize]));
            } else if pad {
                out.push('=');
            }
        }
    }
    out
}

fn decode_with(alphabet: &[u8; 64], text: &str, padded: bool) -> Result<Vec<u8>, Error> {
    let bytes = text.as_bytes();
    let body = if padded {
        if !bytes.len().is_multiple_of(4) {
            return Err(Error::Encoding);
        }
        let pads = bytes.iter().rev().take_while(|b| **b == b'=').count();
        if pads > 2 {
            return Err(Error::Encoding);
        }
        &bytes[..bytes.len() - pads]
    } else {
        bytes
    };
    // One character left over carries six bits: not even one byte.
    if body.len() % 4 == 1 {
        return Err(Error::Encoding);
    }
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let mut bits = 0u32;
    let mut held = 0u32;
    for byte in body {
        let value = alphabet
            .iter()
            .position(|a| a == byte)
            .ok_or(Error::Encoding)?;
        bits = (bits << 6 | value as u32) & 0xffff;
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push((bits >> held & 0xff) as u8);
        }
    }
    // What is left over must be zero, or two texts would decode alike.
    if bits & ((1 << held) - 1) != 0 {
        return Err(Error::Encoding);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{std_padded, url};
    use crate::Error;

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
            assert_eq!(std_padded::encode(plain.as_bytes()), coded, "{plain:?}");
            assert_eq!(
                std_padded::decode(coded).unwrap(),
                plain.as_bytes(),
                "{coded:?}"
            );
            let unpadded = coded.trim_end_matches('=');
            assert_eq!(url::encode(plain.as_bytes()), unpadded, "{plain:?}");
            assert_eq!(
                url::decode(unpadded).unwrap(),
                plain.as_bytes(),
                "{unpadded:?}"
            );
        }
        assert_eq!(std_padded::encode(&[0xfb, 0xff]), "+/8=");
        assert_eq!(url::encode(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn each_decoder_refuses_the_other_spelling_and_loose_text() {
        assert_eq!(url::decode("+/8"), Err(Error::Encoding));
        assert_eq!(url::decode("Zg=="), Err(Error::Encoding));
        assert_eq!(std_padded::decode("-_8="), Err(Error::Encoding));
        assert_eq!(std_padded::decode("Zg"), Err(Error::Encoding));
        assert_eq!(std_padded::decode("Z==="), Err(Error::Encoding));
        assert_eq!(std_padded::decode("Zg=a"), Err(Error::Encoding));
        assert_eq!(url::decode("Z"), Err(Error::Encoding));
        // "Zh" would decode to "f" too if the spare bits were ignored.
        assert_eq!(url::decode("Zh"), Err(Error::Encoding));
        assert_eq!(std_padded::decode("Zh=="), Err(Error::Encoding));
    }

    #[test]
    fn every_byte_value_survives_both_spellings() {
        let all: Vec<u8> = (0..=255).collect();
        for len in 0..all.len() {
            let bytes = &all[..len];
            assert_eq!(url::decode(&url::encode(bytes)).unwrap(), bytes);
            assert_eq!(
                std_padded::decode(&std_padded::encode(bytes)).unwrap(),
                bytes
            );
        }
        assert_eq!(url::decode_array::<2>("-_8"), Ok([0xfb, 0xff]));
        assert_eq!(url::decode_array::<3>("-_8"), Err(Error::Length));
    }
}
