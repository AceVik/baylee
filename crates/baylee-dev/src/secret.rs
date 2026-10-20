//! Throwaway secrets for local fixtures, from the operating system's RNG.
use anyhow::{Result, anyhow};
use std::fmt::Write;

/// `bytes` random bytes as lowercase hex.
pub fn hex(bytes: usize) -> Result<String> {
    let mut buf = vec![0_u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| anyhow!("no randomness: {e}"))?;
    Ok(buf
        .iter()
        .fold(String::with_capacity(bytes * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_secrets_have_the_asked_length_and_differ() {
        let (a, b) = (super::hex(32).unwrap(), super::hex(32).unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
