mod merge;
mod source;
use super::{publish::ASSETS, verify_index};
use crate::{files, trajectory::cache};
use anyhow::{Context, Result, ensure};
use clap::Args;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf};
#[derive(Args)]
/// Recover closed complete-game shards from one explicitly memory-quarantined attempt.
pub struct Recovery {
    /// Preserved memory-quarantined conversion receipt.
    #[arg(long)]
    pub failed_job: PathBuf,
    /// Completed matching converter receipt.
    #[arg(long)]
    pub template_job: PathBuf,
    /// Fresh package destination.
    #[arg(long)]
    pub out: PathBuf,
    /// Maximum number of evenly spread closed shards.
    #[arg(long, default_value_t = 8)]
    pub shards: usize,
    /// Maximum compressed shard size admitted to recovery.
    #[arg(long, default_value_t = 100)]
    pub max_shard_mib: u64,
}
/// Preserve the failed attempt, exclude its last shard and recover only verified training histories.
/// # Errors
/// Rejects non-memory failures, changed tools, split overlap and corrupt games.
pub fn recover(args: &Recovery) -> Result<Value> {
    let (failed, template) = source::checked_jobs(args)?;
    let source = PathBuf::from(source::argument(&failed, "--out")?);
    let base = PathBuf::from(source::argument(&template, "--out")?);
    let (train_runs, provenance) = source::game_sources(&failed)?;
    let allowed = provenance.keys().copied().collect();
    let (all, selected) = source::select_shards(args, &source)?;
    fs::create_dir(&args.out)?;
    fs::create_dir(args.out.join("train"))?;
    let directory = args.out.join(".trajectory-cache-v2");
    fs::create_dir(&directory)?;
    let mut parts = Vec::new();
    let mut games = BTreeSet::new();
    let mut frames = 0_u64;
    let mut shards = Vec::new();
    for (i, path) in selected.iter().enumerate() {
        let data = directory.join(format!("part-{i}.bin"));
        let index = directory.join(format!("part-{i}.idx"));
        let records = cache::build_part(path, &data, &index)?;
        let (local, checked) = verify_index(&index, &allowed)?;
        ensure!(
            records == checked && local.is_disjoint(&games),
            "game spans shards, duplicate game or frame mismatch"
        );
        games.extend(local);
        frames = frames
            .checked_add(records)
            .context("recovered frame overflow")?;
        let name = path
            .file_name()
            .context("shard name")?
            .to_str()
            .context("UTF8 shard name")?
            .to_owned();
        fs::hard_link(path, args.out.join("train").join(&name))?;
        shards.push(name);
        parts.push((path.clone(), data, index));
    }
    for asset in ASSETS {
        if source.join(asset).exists() {
            ensure!(
                files::sha256(&source.join(asset))? == files::sha256(&base.join(asset))?,
                "partial conversion asset differs from completed template"
            );
        }
        fs::hard_link(base.join(asset), args.out.join(asset))?;
    }
    let mut meta = files::read_json(&base.join("meta.json"))?;
    let excluded = all
        .iter()
        .filter(|p| !selected.contains(p))
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    let updates = json!({"train_shards":shards,"val_shards":[],"train_steps":frames,"val_steps":0,"games_requested":games.len(),"games_converted":games.len(),"games_rejected":0,"source_runs":train_runs,"validation_runs":[],"unencoded_decisions":0,"coverage_matrix":null,"recovery":{"source_job":args.failed_job,"source_job_sha256":files::sha256(&args.failed_job)?,"template_job":args.template_job,"game_sources":games.iter().map(|g| provenance[g].clone()).collect::<Vec<_>>(),"excluded_shards":excluded,"scope":"Complete checked training subset; use separate immutable validation; not the full corpus"}});
    meta.as_object_mut()
        .context("template metadata object")?
        .extend(
            updates
                .as_object()
                .context("recovery update object")?
                .clone(),
        );
    files::atomic_json(&args.out.join("meta.json"), &meta)?;
    merge::merge(&parts, &args.out, &shards, frames)?;
    Ok(json!({"status":"ready","games":games.len(),"frames":frames,"path":args.out}))
}
