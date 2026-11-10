//! Copies a deck manifest with some of its decks moved to another split, and
//! nothing else changed: the way a split is carved out of another (the decks
//! no game of a model's lineage seated, out of `train` into `holdout2`).
//!
//! ```text
//! manifest_split4 --manifest manifest.v4.json --keys-from pools/manifest-unseen-train.json \
//!     --split holdout2 --out manifest.v4.holdout2.json
//! ```
//!
//! The keys come from a manifest (its decks' `id`s, as `seated_decks4
//! --pools-dir` writes them) or a JSON list. Every key must name a deck of
//! `--from-split`. The copy is written once, read-only, and checked against
//! the original: every entry but the moved decks' `split` is the same.
use anyhow::{Context, Result, bail, ensure};
use baylee_train_ops::files::sha256;
use clap::Parser;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "Copies a deck manifest with some decks moved to another split")]
struct Args {
    /// The manifest copied; it is never written.
    #[arg(long)]
    manifest: PathBuf,
    /// The decks moved: a manifest (its `decks[].id`) or a JSON list of keys.
    #[arg(long)]
    keys_from: PathBuf,
    /// The split they move to.
    #[arg(long)]
    split: String,
    /// The split every moved deck must be in now.
    #[arg(long, default_value = "train")]
    from_split: String,
    /// The copy; it must not exist yet.
    #[arg(long)]
    out: PathBuf,
}

/// The keys `value` names: a manifest's deck ids, or a list of keys.
fn keys_of(value: &Value) -> Result<BTreeSet<String>> {
    let list = value["decks"]
        .as_array()
        .map(|decks| decks.iter().map(|deck| &deck["id"]).collect::<Vec<_>>())
        .or_else(|| value.as_array().map(|keys| keys.iter().collect()))
        .context("--keys-from is neither a manifest nor a list of keys")?;
    list.into_iter()
        .map(|key| {
            key.as_str()
                .map(str::to_owned)
                .context("a key that is not a string")
        })
        .collect()
}

/// `manifest` with the decks `keys` names moved from `from` to `to`.
///
/// # Errors
/// A key the manifest does not hold, or holds in another split than `from`.
fn moved(manifest: &Value, keys: &BTreeSet<String>, from: &str, to: &str) -> Result<Value> {
    ensure!(from != to, "the decks are in {to} already");
    let mut copy = manifest.clone();
    let mut found = BTreeSet::new();
    for deck in copy["decks"]
        .as_array_mut()
        .context("a manifest without decks")?
    {
        let Some(id) = deck["id"].as_str().map(str::to_owned) else {
            continue;
        };
        if !keys.contains(&id) {
            continue;
        }
        ensure!(
            deck["split"] == from,
            "{id} is in the {} split, not {from}",
            deck["split"]
        );
        deck["split"] = Value::String(to.to_owned());
        found.insert(id);
    }
    let missing: Vec<&String> = keys.difference(&found).collect();
    if !missing.is_empty() {
        bail!("the manifest holds no deck {missing:?}");
    }
    Ok(copy)
}

/// Checks that `copy` is `original` with exactly the decks `keys` names in
/// split `to`, and every other field the same.
fn verify(original: &Value, copy: &Value, keys: &BTreeSet<String>, to: &str) -> Result<()> {
    let mut expected = original.clone();
    for deck in expected["decks"].as_array_mut().context("no decks")? {
        if deck["id"].as_str().is_some_and(|id| keys.contains(id)) {
            deck["split"] = Value::String(to.to_owned());
        }
    }
    ensure!(
        &expected == copy,
        "the copy differs from the original elsewhere"
    );
    Ok(())
}

/// The moved decks counted by format.
fn by_format(manifest: &Value, keys: &BTreeSet<String>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for deck in manifest["decks"].as_array().into_iter().flatten() {
        if deck["id"].as_str().is_some_and(|id| keys.contains(id)) {
            *counts
                .entry(deck["format"].as_str().unwrap_or("?").to_owned())
                .or_default() += 1;
        }
    }
    counts
}

fn write_once_read_only(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write as _;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("{} exists or cannot be created", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let mut permissions = file.metadata()?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let raw =
        fs::read(&args.manifest).with_context(|| format!("reading {}", args.manifest.display()))?;
    let original: Value = serde_json::from_slice(&raw)?;
    let keys = keys_of(&serde_json::from_slice(&fs::read(&args.keys_from)?)?)?;
    ensure!(!keys.is_empty(), "no deck to move");
    let copy = moved(&original, &keys, &args.from_split, &args.split)?;
    verify(&original, &copy, &keys, &args.split)?;
    // Written as the original is (pretty, no trailing newline), so a diff of
    // the two files shows the moved splits and nothing else when the
    // original round-trips.
    let same_layout = serde_json::to_vec_pretty(&original)? == raw;
    let bytes = serde_json::to_vec_pretty(&copy)?;
    write_once_read_only(&args.out, &bytes)?;
    let written: Value = serde_json::from_slice(&fs::read(&args.out)?)?;
    verify(&original, &written, &keys, &args.split)?;
    println!(
        "{} decks moved from {} to {} ({:?}); {} sha256 {}{}",
        keys.len(),
        args.from_split,
        args.split,
        by_format(&original, &keys),
        args.out.display(),
        sha256(&args.out)?,
        if same_layout {
            ""
        } else {
            " (the original does not round-trip: the layout differs, the content does not)"
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest() -> Value {
        json!({"counts_by_split": {"train": 3}, "decks": [
            {"id": "a", "split": "train", "format": "constructed", "path": "a.txt"},
            {"id": "b", "split": "train", "format": "commander", "path": "b.txt"},
            {"id": "c", "split": "test", "format": "constructed", "path": "c.txt"},
            {"id": "d", "split": "train", "format": "constructed", "path": "d.txt"},
        ]})
    }

    fn keys(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|key| (*key).to_owned()).collect()
    }

    #[test]
    fn only_the_named_decks_move_and_nothing_else_changes() {
        let original = manifest();
        let copy = moved(&original, &keys(&["a", "b"]), "train", "holdout2").unwrap();
        assert_eq!(copy["decks"][0]["split"], "holdout2");
        assert_eq!(copy["decks"][1]["split"], "holdout2");
        assert_eq!(copy["decks"][2], original["decks"][2]);
        assert_eq!(copy["decks"][3], original["decks"][3]);
        assert_eq!(copy["counts_by_split"], original["counts_by_split"]);
        verify(&original, &copy, &keys(&["a", "b"]), "holdout2").unwrap();
        assert!(verify(&original, &copy, &keys(&["a"]), "holdout2").is_err());
        assert_eq!(
            by_format(&original, &keys(&["a", "b"])),
            BTreeMap::from([("commander".to_owned(), 1), ("constructed".to_owned(), 1)])
        );
    }

    #[test]
    fn a_deck_of_another_split_or_none_is_refused() {
        let original = manifest();
        let error = moved(&original, &keys(&["c"]), "train", "holdout2").unwrap_err();
        assert!(error.to_string().contains("c is in"), "{error}");
        let error = moved(&original, &keys(&["a", "zz"]), "train", "holdout2").unwrap_err();
        assert!(error.to_string().contains("zz"), "{error}");
    }

    #[test]
    fn keys_come_from_a_manifest_or_a_list() {
        assert_eq!(keys_of(&manifest()).unwrap(), keys(&["a", "b", "c", "d"]));
        assert_eq!(keys_of(&json!(["x", "y"])).unwrap(), keys(&["x", "y"]));
        assert!(keys_of(&json!({"other": 1})).is_err());
    }

    #[test]
    fn the_copy_is_written_once_and_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copy.json");
        write_once_read_only(&path, b"{}").unwrap();
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
        assert!(write_once_read_only(&path, b"{}").is_err());
    }
}
