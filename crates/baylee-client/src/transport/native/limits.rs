//! Bounded response reads, independent of server-declared length or transfer encoding.

use std::io::Read;

pub(super) fn read_body(reader: impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let probe = u64::try_from(limit)
        .ok()
        .and_then(|limit| limit.checked_add(1))
        .ok_or("invalid response size limit")?;
    let mut bytes = Vec::new();
    reader
        .take(probe)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("Failed to read response body: {err}"))?;
    if bytes.len() > limit {
        return Err("response body exceeds its size limit".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::read_body;
    use std::io::{Cursor, Read};

    #[test]
    fn response_limit_accepts_exact_boundary_and_stops_one_byte_after_it() {
        for limit in [0, 1, 4096, 2 * 1024 * 1024] {
            let bytes = vec![42; limit];
            assert_eq!(read_body(Cursor::new(&bytes), limit).unwrap(), bytes);
            let mut endless = std::io::repeat(42).take(u64::MAX);
            assert!(read_body(&mut endless, limit).is_err());
            assert_eq!(endless.limit(), u64::MAX - (limit as u64 + 1));
        }
    }

    #[test]
    fn partial_reader_failure_never_publishes_a_truncated_response() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "cut",
                ))
            }
        }
        let reader = Cursor::new(b"prefix").chain(Broken);
        assert!(read_body(reader, 64).is_err());
    }
}
