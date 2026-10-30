//! Strict research tensors and dedicated input admission; never ordinary PPO data.
pub mod admission;
mod arrays;
mod catalog;
mod frame;
mod session;
#[cfg(all(test, target_os = "linux"))]
mod tests;
pub use arrays::Batch;
pub use session::{Session, serve};

use anyhow::{Context as _, Result, ensure};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::path::Path;

/// Separate tensor protocol identity; incompatible with ordinary packed targets.
pub const SCHEMA: &str = "failure_terminal_research_tensors_v1";
/// Frozen source baseline without advantage centering; never inferred from ordinary PPO.
pub const OBJECTIVE: &str = "failure_terminal_utility_minus_frozen_source_value_uncentered_v1";
const MAX_BYTES: usize = 512 * 1024 * 1024;

fn uint(value: &Value) -> Result<usize> {
    usize::try_from(value.as_u64().context("missing unsigned tensor field")?).map_err(Into::into)
}
fn array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value]> {
    value[name]
        .as_array()
        .map(Vec::as_slice)
        .with_context(|| format!("missing tensor array {name}"))
}
fn sha(bytes: &[u8]) -> String {
    crate::files::hex(&Sha256::digest(bytes))
}
fn pinned(path: &Path, expected: &str) -> Result<()> {
    ensure!(
        expected.len() == 64
            && expected
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid external SHA256"
    );
    ensure!(
        path.is_absolute()
            && path.canonicalize()? == path
            && std::fs::symlink_metadata(path)?.file_type().is_file(),
        "pin must name canonical regular file"
    );
    ensure!(
        crate::files::sha256(path)? == expected,
        "research pin changed: {}",
        path.display()
    );
    Ok(())
}
