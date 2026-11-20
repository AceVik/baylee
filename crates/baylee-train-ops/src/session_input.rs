//! Bounded input for sessions issued by a private evaluation gateway.
use std::io::{BufRead, Read as _};
use std::net::SocketAddr;

use anyhow::{Result, ensure};
use serde::de::DeserializeOwned;

/// Check the address before forwarding any credential.
///
/// # Errors
/// Requires a local HTTP gateway with a literal loopback address and port.
pub fn require_loopback(gateway: &str) -> Result<()> {
    ensure!(
        gateway
            .trim_end_matches('/')
            .strip_prefix("http://")
            .and_then(|address| address.parse::<SocketAddr>().ok())
            .is_some_and(|address| address.ip().is_loopback()),
        "session handoff requires an explicit loopback HTTP gateway and port"
    );
    Ok(())
}

/// Read one complete newline-delimited object without quoting input in errors.
///
/// # Errors
/// When the line is truncated, oversized or invalid.
pub fn read_line<T: DeserializeOwned>(input: impl BufRead) -> Result<T> {
    const MAX_BYTES: u64 = 16_384;
    let mut bytes = Vec::new();
    input.take(MAX_BYTES + 1).read_until(b'\n', &mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES && bytes.last() == Some(&b'\n'),
        "invalid length or incomplete session handoff"
    );
    serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid session handoff"))
}

/// # Errors
/// When a token cannot be used in a bearer header.
pub fn validate_token(token: &str) -> Result<()> {
    ensure!(
        !token.is_empty()
            && token.len() <= 4096
            && token.bytes().all(|byte| byte.is_ascii_graphic()),
        "invalid credential in session handoff"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_nonlocal_destinations_and_incomplete_or_oversized_secrets() {
        assert!(require_loopback("http://127.0.0.1:28769").is_ok());
        assert!(require_loopback("http://127.0.0.1:28769@example.com").is_err());
        assert!(require_loopback("http://127.0.0.1:28769/redir").is_err());
        assert!(read_line::<String>(&b"\"sample\""[..]).is_err());
        assert!(read_line::<String>(vec![b'A'; 16_385].as_slice()).is_err());
        assert!(validate_token("value\rheader").is_err());
        assert!(
            !read_line::<String>(&b"{\"sensitive\": true}\n"[..])
                .unwrap_err()
                .to_string()
                .contains("sensitive")
        );
    }
}
