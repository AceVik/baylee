//! Independent evidence from disclosed Astra and Terra TRAIN seats.

mod game;
mod objects;
mod recovery;
mod seat;
use baylee_cards_pool as _;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use flate2::read::MultiGzDecoder;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    attempt: PathBuf,
    #[arg(long)]
    decks: PathBuf,
    #[arg(long)]
    train_manifest: PathBuf,
    /// SHA-256 of the frozen TRAIN deck manifest the games were planned on.
    #[arg(long)]
    train_manifest_sha256: String,
    /// Full commit of the build that played the games.
    #[arg(long)]
    engine_commit: String,
    #[arg(long)]
    out: PathBuf,
    /// Create a separate collector snapshot with the verified offline receipt.
    #[arg(long)]
    recover_collector: Option<PathBuf>,
}

const ASTRA: &str = "gpt-6.2-astra";
const TERRA: &str = "gpt-6.1-terra";

/// What the audited games must have been played on.
struct Pins<'a> {
    build: &'a str,
    manifest_sha256: &'a str,
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.engine_commit.len() == 40
            && args
                .engine_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "--engine-commit needs a full commit id"
    );
    let pins = Pins {
        build: &args.engine_commit,
        manifest_sha256: &args.train_manifest_sha256,
    };
    let attempt = args.attempt.canonicalize()?;
    ensure!(
        hash(&fs::read(&args.train_manifest)?) == args.train_manifest_sha256,
        "TRAIN manifest changed"
    );
    let report = game::audit(&attempt, &args.decks, &pins)?;
    let contents = serde_json::to_vec_pretty(&report)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.out)?;
    output.write_all(&contents)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    if let Some(collector) = &args.recover_collector {
        recovery::publish(&attempt, collector, &report, pins.build)?;
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("reading {}", path.display()))
}

fn json_file(path: &Path) -> Result<Value> {
    serde_json::from_slice(&read(path)?).with_context(|| format!("parsing {}", path.display()))
}

fn lines<T: DeserializeOwned>(bytes: &[u8]) -> Result<Vec<T>> {
    let data = unpack(bytes)?;
    std::str::from_utf8(&data)?
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).with_context(|| format!("JSONL line {}", index + 1))
        })
        .collect()
}

fn unpack(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }
    let mut decoded = Vec::new();
    MultiGzDecoder::new(bytes).read_to_end(&mut decoded)?;
    Ok(std::borrow::Cow::Owned(decoded))
}

fn hash(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(text, "{byte:02x}").expect("String");
    }
    text
}

fn number(value: &Value, field: &str) -> Result<u64> {
    value[field]
        .as_u64()
        .with_context(|| format!("Missing {field}"))
}

fn menu_body(prompt: &str) -> Result<&str> {
    prompt
        .split_once("\nQUESTION:")
        .map(|(_, text)| text.trim_end())
        .context("Missing QUESTION section")
}
