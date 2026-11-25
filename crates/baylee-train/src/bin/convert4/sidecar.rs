//! The files of a sidecar beside a dataset, written together: the data files
//! first and the header, which names their digests, last, so a directory with
//! a header holds the whole sidecar.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, ensure};
use serde_json::Value;

/// The header's file name.
const HEADER: &str = "header.json";

/// A sidecar as written.
pub(super) struct Sidecar {
    pub(super) header: Value,
    /// Each file's SHA-256 by name, the header's among them: what `meta.json`
    /// says the sidecar of its dataset is.
    pub(super) files_sha256: BTreeMap<String, String>,
}

/// Writes `files`, then `header`, into `dir`.
///
/// # Errors
/// When `dir` holds one of the files already, or one cannot be written.
pub(super) fn write(dir: &Path, files: &[(&str, &[u8])], header: Value) -> anyhow::Result<Sidecar> {
    let bytes = serde_json::to_vec_pretty(&header)?;
    let files: Vec<(&str, &[u8])> = files
        .iter()
        .copied()
        .chain(std::iter::once((HEADER, bytes.as_slice())))
        .collect();
    fs::create_dir_all(dir)?;
    for (name, _) in &files {
        let path = dir.join(name);
        ensure!(!path.exists(), "{} exists", path.display());
    }
    let mut files_sha256 = BTreeMap::new();
    for (name, bytes) in files {
        let path = dir.join(name);
        fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        files_sha256.insert(name.to_owned(), super::shadow::sha(bytes));
    }
    Ok(Sidecar {
        header,
        files_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_sidecar_names_every_files_digest_and_is_never_written_over() {
        let directory = std::env::temp_dir().join(format!("sidecar-{}", uuid::Uuid::now_v7()));
        let written = write(
            &directory,
            &[("index.bin", b"index"), ("cands.bin", b"")],
            json!({"rows": 2}),
        )
        .unwrap();
        for (name, digest) in &written.files_sha256 {
            assert_eq!(
                *digest,
                crate::shadow::sha(&fs::read(directory.join(name)).unwrap()),
                "{name}"
            );
        }
        assert_eq!(
            written.files_sha256.keys().collect::<Vec<_>>(),
            ["cands.bin", "header.json", "index.bin"]
        );
        let header: Value =
            serde_json::from_slice(&fs::read(directory.join(HEADER)).unwrap()).unwrap();
        assert_eq!(header, written.header);
        assert!(write(&directory, &[("index.bin", b"other")], json!({})).is_err());
        assert_eq!(fs::read(directory.join("index.bin")).unwrap(), b"index");
        fs::remove_dir_all(&directory).unwrap();
    }
}
