//! Bounded private pipe framing; lengths are checked before allocating.
use std::io::{self, Read, Write};

/// Writes one length-prefixed frame, refusing an oversized or empty payload.
///
/// # Errors
/// Invalid length or an underlying pipe write/flush failure.
pub fn write(writer: &mut impl Write, bytes: &[u8], limit: usize) -> io::Result<()> {
    let length = u32::try_from(bytes.len())
        .ok()
        .filter(|length| *length != 0 && bytes.len() <= limit)
        .ok_or_else(invalid)?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()
}

/// Reads one complete bounded frame. EOF, including a partial frame, is an error.
///
/// # Errors
/// Invalid length, early EOF or a pipe read failure.
pub fn read(reader: &mut impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix)?;
    let length = length(prefix, limit)?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// Checks a four-byte length before a synchronous or asynchronous allocation.
///
/// # Errors
/// Empty, oversized or unrepresentable lengths.
pub fn length(prefix: [u8; 4], limit: usize) -> io::Result<usize> {
    usize::try_from(u32::from_be_bytes(prefix))
        .ok()
        .filter(|length| *length != 0 && *length <= limit)
        .ok_or_else(invalid)
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid private worker frame length",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_empty_and_truncated_frames_are_refused_without_reading_a_body() {
        struct PrefixOnly(&'static [u8]);
        impl Read for PrefixOnly {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                assert!(
                    !self.0.is_empty(),
                    "a refused length must not read or allocate its body"
                );
                self.0.read(out)
            }
        }
        assert!(read(&mut PrefixOnly(&[255; 4]), 16).is_err());
        assert!(read(&mut PrefixOnly(&[0; 4]), 16).is_err());
        assert!(read(&mut &[0, 0, 0, 4, 1][..], 16).is_err());
        let mut pipe = Vec::new();
        assert!(write(&mut pipe, &[1; 17], 16).is_err());
        assert!(write(&mut pipe, &[], 16).is_err());
        assert!(pipe.is_empty());
        write(&mut pipe, &[1; 16], 16).unwrap();
        assert_eq!(read(&mut pipe.as_slice(), 16).unwrap(), [1; 16]);
    }
}
