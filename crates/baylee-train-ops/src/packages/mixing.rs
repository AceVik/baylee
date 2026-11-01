use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
};

fn order(seed: u64, key: &str) -> Vec<u8> {
    Sha256::digest(format!("{seed}:{key}")).to_vec()
}

/// Mix whole games by available frame share, independent of readiness listing order.
/// # Errors
/// Rejects duplicate identifiers, malformed entries and overflowing frame totals.
pub fn select_entries(pending: &[Value], frame_limit: u64, seed: u64) -> Result<Vec<Value>> {
    ensure!(frame_limit > 0, "positive package frame limit required");
    let mut groups = BTreeMap::<String, Vec<Value>>::new();
    let mut ids = BTreeSet::new();
    for entry in pending {
        ensure!(
            entry["steps"].as_u64().is_some_and(|n| n > 0),
            "only nonempty completed games enter packages"
        );
        let id = entry["game_id"].as_u64().context("missing game ID")?;
        ensure!(ids.insert(id), "duplicate pending game ID");
        let source = Path::new(
            entry["source_shard"]
                .as_str()
                .context("missing source shard")?,
        )
        .parent()
        .and_then(Path::parent)
        .context("source shard needs source/run parents")?
        .to_string_lossy()
        .into_owned();
        groups.entry(source).or_default().push(entry.clone());
    }
    let mut queues = BTreeMap::<String, VecDeque<Value>>::new();
    let mut available = BTreeMap::new();
    let mut consumed = BTreeMap::<String, u64>::new();
    for (source, mut rows) in groups {
        rows.sort_by_key(|row| {
            (
                order(seed, &row["game_id"].to_string()),
                row["game_id"].as_u64(),
            )
        });
        let frames = rows.iter().try_fold(0_u64, |n, row| {
            n.checked_add(row["steps"].as_u64().context("frame count")?)
                .context("source frame overflow")
        })?;
        available.insert(source.clone(), frames);
        consumed.insert(source.clone(), 0);
        queues.insert(source, rows.into());
    }
    let mut selected = Vec::new();
    let mut frames = 0_u64;
    while !queues.is_empty() && frames < frame_limit {
        let source = queues
            .keys()
            .min_by(|a, b| {
                let a_next = consumed[*a] + queues[*a][0]["steps"].as_u64().unwrap_or(0);
                let b_next = consumed[*b] + queues[*b][0]["steps"].as_u64().unwrap_or(0);
                (u128::from(a_next) * u128::from(available[*b]))
                    .cmp(&(u128::from(b_next) * u128::from(available[*a])))
                    .then_with(|| order(seed, a).cmp(&order(seed, b)))
                    .then_with(|| a.cmp(b))
            })
            .context("pending source")?
            .clone();
        let queue = queues.get_mut(&source).context("source queue")?;
        let entry = queue.pop_front().context("source game")?;
        let count = entry["steps"].as_u64().context("game frames")?;
        frames = frames
            .checked_add(count)
            .context("package frame overflow")?;
        *consumed.get_mut(&source).context("source consumed")? += count;
        if queue.is_empty() {
            queues.remove(&source);
        }
        selected.push(entry);
    }
    Ok(selected)
}
