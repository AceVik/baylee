//! `--llm-bc-skip-unreplayable`: LLM games this build cannot replay.
//!
//! An LLM-BC conversion publishes only when every selected game converts.
//! A game recorded on an older build stops converting when the engine's
//! rules for one of its cards change: the replay audit refuses an input or
//! finds a different table, so its labels would teach decisions taken under
//! rules this build no longer plays. Such a game may be left out, and only
//! such a game: the list is the `rejected-games.json` a failed conversion
//! of the same runs wrote, every entry must carry a replay-audit reason,
//! and every entry must name a game of the runs (a stale list is refused).
//! Selection never looks at an outcome. The dataset records what it left
//! out, with the reasons and the list's digest.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

/// The reason prefix the replay audit writes (`convert4::streaming`).
const REPLAY_AUDIT: &str = "seen replay audit:";

/// One rejected game as `rejected-games.json` lists it.
#[derive(Deserialize)]
struct Rejected {
    shard: PathBuf,
    offset: u64,
    len: u64,
    reason: String,
}

/// The games to leave out, by record location, and which of them were met.
pub(super) struct Skip {
    list_sha256: String,
    reasons: BTreeMap<(PathBuf, u64, u64), String>,
    skipped: Vec<Value>,
}

impl Skip {
    /// Reads a failed conversion's `rejected-games.json`.
    pub(super) fn load(path: &Path) -> anyhow::Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let rows: Vec<Rejected> = serde_json::from_slice(&bytes)
            .with_context(|| format!("{} is not a rejected-games list", path.display()))?;
        ensure!(!rows.is_empty(), "{} lists no game", path.display());
        let mut reasons = BTreeMap::new();
        for row in rows {
            ensure!(
                row.reason.starts_with(REPLAY_AUDIT),
                "only games this build cannot replay may be skipped, not: {}",
                row.reason
            );
            let shard = row
                .shard
                .canonicalize()
                .with_context(|| format!("listed record {}", row.shard.display()))?;
            ensure!(
                reasons
                    .insert((shard, row.offset, row.len), row.reason)
                    .is_none(),
                "{} lists a game twice",
                path.display()
            );
        }
        Ok(Self {
            list_sha256: crate::shadow::sha(&bytes),
            reasons,
            skipped: Vec::new(),
        })
    }

    /// Whether the game at this record location is listed; a listed game is
    /// remembered as skipped.
    pub(super) fn takes(&mut self, shard: &Path, offset: u64, len: u64) -> bool {
        let key = (shard.to_path_buf(), offset, len);
        let Some(reason) = self.reasons.get(&key) else {
            return false;
        };
        self.skipped.push(json!({
            "shard": shard,
            "offset": offset,
            "len": len,
            "reason": reason,
        }));
        true
    }

    /// The dataset's record of what it left out; refuses a list naming a
    /// game the runs do not hold.
    pub(super) fn metadata(&self) -> anyhow::Result<Value> {
        ensure!(
            self.skipped.len() == self.reasons.len(),
            "the skip list names {} games, the runs hold {} of them",
            self.reasons.len(),
            self.skipped.len()
        );
        Ok(json!({
            "rule": "replay_audit_refused_on_this_build",
            "list_sha256": self.list_sha256,
            "games": self.skipped,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A fresh directory under the system temp dir, removed with the guard.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "baylee-unreplayable-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn list(dir: &Path, rows: &Value) -> PathBuf {
        let path = dir.join("rejected-games.json");
        fs::write(&path, serde_json::to_vec(rows).unwrap()).unwrap();
        path
    }

    fn record(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, b"record").unwrap();
        path.canonicalize().unwrap()
    }

    #[test]
    fn a_listed_replay_refusal_is_skipped_and_recorded() {
        let dir = TempDir::new();
        let shard = record(dir.path(), "a.jsonl.gz");
        let path = list(
            dir.path(),
            &json!([{"game_id": 3, "shard": shard, "offset": 0, "len": 10,
                     "reason": "seen replay audit: input 7 refused"}]),
        );
        let mut skip = Skip::load(&path).unwrap();
        assert!(
            !skip.takes(&shard, 0, 11),
            "another record at the same path"
        );
        assert!(skip.takes(&shard, 0, 10));
        let meta = skip.metadata().unwrap();
        assert_eq!(
            meta["games"][0]["reason"],
            "seen replay audit: input 7 refused"
        );
        assert_eq!(meta["list_sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn only_replay_refusals_may_be_skipped() {
        let dir = TempDir::new();
        let shard = record(dir.path(), "a.jsonl.gz");
        let path = list(
            dir.path(),
            &json!([{"shard": shard, "offset": 0, "len": 10,
                     "reason": "label does not match the executed action"}]),
        );
        assert!(Skip::load(&path).is_err());
    }

    #[test]
    fn a_list_naming_a_game_the_runs_lack_is_refused() {
        let dir = TempDir::new();
        let shard = record(dir.path(), "a.jsonl.gz");
        let path = list(
            dir.path(),
            &json!([{"shard": shard, "offset": 0, "len": 10,
                     "reason": "seen replay audit: table differs after input 3"}]),
        );
        let skip = Skip::load(&path).unwrap();
        assert!(skip.metadata().is_err());
    }
}
