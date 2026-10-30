use super::Recovery;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

pub(super) fn argument<'a>(job: &'a Value, name: &str) -> Result<&'a str> {
    let command = job["command"].as_array().context("conversion command")?;
    let at = command
        .iter()
        .position(|v| v == name)
        .with_context(|| format!("missing conversion {name}"))?;
    command
        .get(at + 1)
        .and_then(Value::as_str)
        .with_context(|| format!("missing {name} value"))
}
pub(super) fn checked_jobs(args: &Recovery) -> Result<(Value, Value)> {
    ensure!(
        args.shards > 0 && args.max_shard_mib > 0,
        "positive recovery limits required"
    );
    let failed = files::read_json(&args.failed_job)?;
    let template = files::read_json(&args.template_job)?;
    ensure!(
        failed["status"] == "quarantined"
            && failed["reason"] == "memory_limit"
            && template["status"] == "complete",
        "expected memory-limited attempt and completed template"
    );
    ensure!(
        failed["executable_sha256"].is_string()
            && failed["executable_sha256"] == template["executable_sha256"],
        "conversion executable differs from template"
    );
    for key in ["--encoder-version", "--max-entities", "--max-candidates"] {
        ensure!(
            argument(&failed, key).ok() == argument(&template, key).ok(),
            "conversion semantic flag differs from template: {key}"
        );
    }
    for line in BufReader::new(File::open(args.failed_job.with_file_name("stderr.log"))?).lines() {
        let line = line?;
        ensure!(
            !line.contains("UNENCODED") && !line.contains("quarantining game"),
            "review replay/encoding failures before recovery"
        );
    }
    Ok((failed, template))
}
pub(super) fn game_sources(failed: &Value) -> Result<(Vec<String>, BTreeMap<u32, Value>)> {
    let train = argument(failed, "--runs")?
        .split(',')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let val = argument(failed, "--val-runs")?
        .split(',')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    ensure!(
        !train.iter().any(|run| val.contains(run)),
        "overlapping source splits"
    );
    ensure!(
        train.iter().chain(&val).collect::<BTreeSet<_>>().len() == train.len() + val.len(),
        "duplicate source run"
    );
    let mut provenance = BTreeMap::new();
    let mut game_id = 0_u32;
    for run in train.iter().chain(&val) {
        for line in BufReader::new(File::open(Path::new(run).join("games.jsonl"))?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let game: Value = serde_json::from_str(&line)?;
            if !matches!(
                game["outcome"]["kind"].as_str(),
                Some("won" | "team_won" | "draw")
            ) {
                continue;
            }
            game_id = game_id.checked_add(1).context("game ID overflow")?;
            if train.contains(run) {
                provenance.insert(game_id,json!({"run":run,"seed":game["seed"],"shard":game["shard"],"offset":game["offset"],"len":game["len"]}));
            }
        }
    }
    Ok((train, provenance))
}
pub(super) fn select_shards(
    args: &Recovery,
    source: &Path,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut all = Vec::new();
    for entry in fs::read_dir(source.join("train"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "bst4")
            || path
                .file_name()
                .is_some_and(|v| v.to_string_lossy().ends_with(".jsonl.gz"))
        {
            all.push(path);
        }
    }
    all.sort();
    let max_bytes = args
        .max_shard_mib
        .checked_mul(1 << 20)
        .context("shard size overflow")?;
    let candidates = all
        .iter()
        .take(all.len().saturating_sub(1))
        .filter_map(|path| match path.metadata() {
            Ok(meta) if meta.len() <= max_bytes => Some(Ok(path.clone())),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(
        !candidates.is_empty(),
        "no closed complete candidate shards"
    );
    let count = args.shards.min(candidates.len());
    let selected = (0..count)
        .map(|i| {
            candidates[if count == 1 {
                0
            } else {
                i * (candidates.len() - 1) / (count - 1)
            }]
            .clone()
        })
        .collect();
    Ok((all, selected))
}
