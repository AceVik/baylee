//! Lossless indexed research-frame storage. Never tensor packing or PPO admission.
mod rows;
mod source;
mod store;
#[cfg(all(test, target_os = "linux"))]
mod tests;

pub use store::{Reader, build, verify};

use crate::files;
use anyhow::{Context as _, Result, ensure};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    fs::{self, File},
    io::{BufRead, Read},
    path::{Component, Path, PathBuf},
};

const CACHE_SCHEMA: &str = "failure_terminal_research_cache_v1";
const MAX_FRAME: u64 = 64 * 1024 * 1024;
const MAX_EPISODE: u64 = 2 * 1024 * 1024 * 1024;
const DATA_MAGIC: &[u8; 8] = b"BRC1\x01\0\0\0";
const INDEX_MAGIC: &[u8; 8] = b"BRI1\x01\0\0\0";
const INDEX_SIZE: u64 = 48;

fn digest(bytes: &[u8]) -> String {
    files::hex(&Sha256::digest(bytes))
}
fn json_digest(value: &Value) -> Result<String> {
    Ok(digest(&serde_json::to_vec(value)?))
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn regular(path: &Path) -> Result<()> {
    ensure!(
        path.is_absolute()
            && path.canonicalize()? == path
            && fs::symlink_metadata(path)?.file_type().is_file(),
        "research evidence must be a canonical regular file"
    );
    Ok(())
}
fn bound(root: &Path, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && Path::new(name)
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
        "invalid research relative path"
    );
    let path = root.join(name);
    regular(&path)?;
    ensure!(path.starts_with(root), "research path escapes root");
    Ok(path)
}
fn read_json(path: &Path) -> Result<Value> {
    regular(path)?;
    ensure!(
        path.metadata()?.len() <= MAX_FRAME,
        "research metadata exceeds size limit"
    );
    files::read_json(path)
}
fn uint(value: &Value) -> Result<u64> {
    value.as_u64().context("missing unsigned research field")
}
fn lines(mut reader: impl BufRead, mut observe: impl FnMut(&[u8]) -> Result<()>) -> Result<()> {
    let mut total = 0_u64;
    loop {
        let mut line = Vec::new();
        let length = (&mut reader)
            .take(MAX_FRAME + 1)
            .read_until(b'\n', &mut line)?;
        if length == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(length)?)
            .context("research byte count overflow")?;
        ensure!(
            u64::try_from(length)? <= MAX_FRAME && total <= MAX_EPISODE && line.ends_with(b"\n"),
            "oversized or truncated research JSON line"
        );
        line.pop();
        ensure!(!line.is_empty(), "empty research JSON line");
        observe(&line)?;
    }
    Ok(())
}
