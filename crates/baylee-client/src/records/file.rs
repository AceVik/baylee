//! Shared bounded admission for kept, archived and explicitly opened replay files.

use std::io::{self, Read};

/// Encoded file limit, independent of gzip's decoded budget. Allows raw JSONL too.
pub(crate) const MAX_FILE_BYTES: usize = baylee_client_core::bugreport::MAX_INFLATED_BYTES;

pub(crate) fn read(path: &std::path::Path) -> io::Result<Vec<u8>> {
    read_open(&mut std::fs::File::open(path)?)
}

pub(super) fn read_open(file: &mut std::fs::File) -> io::Result<Vec<u8>> {
    // Preflight avoids allocating the budget for a huge existing file. The
    // streaming check remains necessary if another process grows it meanwhile.
    if file.metadata()?.len() > MAX_FILE_BYTES as u64 {
        return Err(too_large());
    }
    read_limit(file, MAX_FILE_BYTES)
}

fn read_limit(reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let probe = u64::try_from(limit)
        .ok()
        .and_then(|limit| limit.checked_add(1))
        .ok_or_else(too_large)?;
    let mut bytes = Vec::new();
    reader.take(probe).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(too_large());
    }
    Ok(bytes)
}

fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "record file exceeds the size limit",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_growing_past_preflight_stops_at_limit_plus_one() {
        for limit in [0, 1, 4096, 2 * 1024 * 1024] {
            let bytes = vec![17; limit];
            assert_eq!(read_limit(bytes.as_slice(), limit).unwrap(), bytes);
            let mut grows = io::repeat(17).take(u64::MAX);
            assert!(read_limit(&mut grows, limit).is_err());
            assert_eq!(grows.limit(), u64::MAX - (limit as u64 + 1));
        }
    }
}
