use super::{publish, select_entries};
use crate::files;
use anyhow::{Context, Result, ensure};
use clap::Args;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

#[derive(Args)]
/// CPU producer; only complete verified training games enter immutable packages.
pub struct Producer {
    /// Streaming converter dataset root.
    #[arg(long)]
    pub source: PathBuf,
    /// Converter status receipt.
    #[arg(long)]
    pub source_job: PathBuf,
    /// Fresh package destination.
    #[arg(long)]
    pub out: PathBuf,
    /// Independent fixed validation dataset.
    #[arg(long)]
    pub validation_data: PathBuf,
    /// Target frames per package, retaining complete games.
    #[arg(long, default_value_t = 500_000)]
    pub frames: u64,
    /// Maximum unconsumed package count.
    #[arg(long, default_value_t = 2)]
    pub ahead: usize,
    /// Stable source-balanced ordering seed.
    #[arg(long, default_value_t = 2718)]
    pub seed: u64,
    /// Maximum producer wall time.
    #[arg(long, default_value_t = 21_600.0)]
    pub timeout: f64,
    /// Permit only already quarantined historical replay-divergence records.
    #[arg(long)]
    pub allow_replay_quarantined: bool,
}

pub(super) fn directories(root: &Path, prefix: &str) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for item in fs::read_dir(root)? {
        let item = item?;
        if item.file_type()?.is_dir() && item.file_name().to_string_lossy().starts_with(prefix) {
            result.push(item.path());
        }
    }
    result.sort();
    Ok(result)
}

fn tick(args: &Producer, header: &Value) -> Result<bool> {
    let job = files::read_json(&args.source_job)?;
    ensure!(
        matches!(
            job["status"].as_str(),
            Some("starting" | "running" | "complete")
        ),
        "converter failed; preserve packages and inspect logs"
    );
    let existing = directories(&args.out, "package-")?;
    let mut used = BTreeSet::new();
    let mut queued = 0;
    for path in &existing {
        ensure!(
            path.join("ready.json").is_file(),
            "preserving incomplete package {}",
            path.display()
        );
        let meta = files::read_json(&path.join("meta.json"))?;
        for id in meta["source_game_ids"]
            .as_array()
            .context("package source IDs")?
        {
            ensure!(
                used.insert(id.as_u64().context("game ID")?),
                "game appears in multiple packages"
            );
        }
        queued += usize::from(!path.join("consumed.json").exists());
    }
    let mut entries = Vec::new();
    let mut paths = fs::read_dir(args.source.join("ready"))?
        .map(|e| e.map(|v| v.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        if path.extension().is_some_and(|ext| ext == "json") {
            let entry = files::read_json(&path)?;
            if entry["split"] == "train"
                && !used.contains(&entry["game_id"].as_u64().context("ready game ID")?)
            {
                entries.push(entry);
            }
        }
    }
    let complete = job["status"] == "complete";
    if complete {
        let final_meta = files::read_json(&args.source.join("meta.json"))?;
        ensure!(
            final_meta["unencoded_decisions"] == 0,
            "review unencoded converter decisions"
        );
        if final_meta["games_rejected"] != 0 {
            ensure!(args.allow_replay_quarantined, "review converter rejections");
            let rejected = files::read_json(&args.source.join("rejected-games.json"))?;
            ensure!(
                rejected
                    .as_array()
                    .context("rejected games")?
                    .iter()
                    .all(|r| r["reason"]
                        .as_str()
                        .is_some_and(|s| s.starts_with("replay audit: Diverged"))),
                "non-divergence converter rejection"
            );
        }
    }
    if complete && entries.is_empty() {
        files::atomic_json(
            &args.out.join("complete.json"),
            &json!({"packages":existing.len(),"games":used.len()}),
        )?;
        return Ok(true);
    }
    let number = u64::try_from(existing.len())?;
    let seed = args
        .seed
        .checked_add(number)
        .context("package seed overflow")?;
    let selected = select_entries(&entries, args.frames, seed)?;
    let frames = selected.iter().try_fold(0_u64, |n, entry| {
        n.checked_add(entry["steps"].as_u64().context("frames")?)
            .context("frame overflow")
    })?;
    if queued < args.ahead && (frames >= args.frames || complete && !selected.is_empty()) {
        let mut package_header = header.clone();
        package_header["package_mixing"] = json!({"version":1,"seed":seed,"strategy":"available frame share per source run; complete games; seeded within-source order"});
        let path = args.out.join(format!("package-{number:04}"));
        let marker = publish(&args.source, &path, &selected, &package_header)?;
        println!("{}", json!({"ready":path,"marker":marker}));
    }
    Ok(false)
}

/// Produce bounded immutable packages without restarting any existing trainer.
/// # Errors
/// Rejects overlap, failed conversions, partial packages and expired/interrupted work.
pub fn produce(args: &Producer, interrupted: &AtomicBool) -> Result<()> {
    ensure!(
        args.ahead > 0 && args.frames > 0 && args.timeout.is_finite() && args.timeout > 0.0,
        "positive package queue/size/deadline required"
    );
    fs::create_dir_all(&args.out)?;
    let _owner = files::lock(&args.out.join("producer.lock"))?;
    let started = Instant::now();
    while !args.source.join("ready").is_dir() {
        ensure!(
            !interrupted.load(Ordering::Relaxed) && started.elapsed().as_secs_f64() < args.timeout,
            "converter header deadline/interruption"
        );
        thread::sleep(Duration::from_secs(2));
    }
    let header = files::read_json(&args.source.join("dataset_header.json"))?;
    let validation = files::read_json(&args.validation_data.join("meta.json"))?;
    ensure!(
        header["structure_sha256"] == validation["structure_sha256"],
        "validation uses a different catalog"
    );
    let training = header["source_runs"]
        .as_array()
        .context("training source runs")?;
    let held_out = validation["validation_runs"]
        .as_array()
        .context("fixed validation runs")?;
    ensure!(
        !training.iter().any(|run| held_out.contains(run)),
        "training source overlaps fixed validation runs"
    );
    loop {
        ensure!(
            !interrupted.load(Ordering::Relaxed) && started.elapsed().as_secs_f64() < args.timeout,
            "package producer deadline/interruption"
        );
        if tick(args, &header)? {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(2));
    }
}
