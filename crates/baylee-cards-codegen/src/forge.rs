//! forge-reference scan: card name → card script path (read-only reference
//! index for card implementation batches; Forge files are never copied).

use crate::error::CodegenError;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Builds the name → script-path index for a forge cardsfolder.
///
/// Paths in the index are relative to `cardsfolder` (e.g. `f/force_of_will.txt`).
///
/// # Errors
/// IO errors while walking or reading the cardsfolder.
pub fn build_index(cardsfolder: &Path) -> Result<BTreeMap<String, String>, CodegenError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    collect(cardsfolder, &mut paths, cardsfolder)?;
    paths.sort();
    let mut map = BTreeMap::new();
    for path in paths {
        let file = fs::File::open(&path).map_err(CodegenError::io(&path))?;
        for line in BufReader::new(file).lines().take(12) {
            let line = line.map_err(CodegenError::io(&path))?;
            if let Some(name) = line.strip_prefix("Name:") {
                let rel = path
                    .strip_prefix(cardsfolder)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                map.entry(name.trim().to_string()).or_insert(rel);
                break;
            }
        }
    }
    Ok(map)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>, root: &Path) -> Result<(), CodegenError> {
    let entries = fs::read_dir(dir).map_err(CodegenError::io(root))?;
    for entry in entries {
        let entry = entry.map_err(CodegenError::io(dir))?;
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out, root)?;
        } else if path.extension().is_some_and(|e| e == "txt") {
            out.push(path);
        }
    }
    Ok(())
}
