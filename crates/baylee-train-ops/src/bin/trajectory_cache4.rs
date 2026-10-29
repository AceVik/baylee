//! Build native trajectory caches and validate teacher-only dataset provenance.
use anyhow::{Context, Result};
use baylee_train_ops::{
    corpus::{bc_cycling, bc_metadata, diagnostic_provenance, training_contracts},
    evaluation::{calibration, diagnostics},
    files,
    trajectory::{cache, contracts, packing},
};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::PathBuf,
};
#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Task,
}
#[derive(Subcommand)]
enum Task {
    /// Build separate lossless research-frame storage; never tensor/PPO data.
    ResearchBuild {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        expected_manifest_sha256: String,
    },
    /// Revalidate immutable research storage and its externally sealed sources.
    ResearchVerify {
        #[arg(long)]
        cache_dir: PathBuf,
        #[arg(long)]
        expected_manifest_sha256: String,
    },
    /// Diagnostic single-frame read after full verification; use Reader sessions for bulk I/O.
    ResearchRead {
        #[arg(long)]
        cache_dir: PathBuf,
        #[arg(long)]
        expected_manifest_sha256: String,
        #[arg(long)]
        row: usize,
    },
    /// Merge and seal archived v3 tensor columns without changing old caches.
    Merge3 {
        #[arg(long)]
        data_dir: PathBuf,
    },
    /// Maintain lock-held reserve/commit/abort around tensor serialization.
    TensorIo,
    /// Resolve feature-cache identity, admission and collection-plan metadata.
    CalibrationIo,
    /// Validate an encoder or recurrent training schedule contract.
    TrainingContract,
    /// Bind immutable source receipts used by tensor diagnostics.
    DiagnosticProvenance,
    /// Aggregate complete tensor prediction arrays on the CPU.
    CalibrationMetrics,
    /// Merge one whole unroll batch of diagnostic sufficient statistics.
    DiagnosticMetrics,
    /// Advance a fixed budget by one complete tensor-sampler source pass.
    BcCycle,
    /// Hash a JSON mixture contract using its existing canonical identity.
    Digest,
    /// Bind observed tensor histories to native source-record metadata.
    GameMetadata {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long)]
        split: String,
    },
    /// Reject overlapping immutable BC source identities.
    RejectOverlap,
    /// Build or reopen a verified immutable trajectory cache.
    Build {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long)]
        split: String,
        #[arg(long)]
        shard: Vec<String>,
    },
    /// Validate BC provenance before reads, or reject it before PPO preparation.
    Validate {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long)]
        policy_gradient: bool,
        #[arg(long)]
        metadata_stdin: bool,
    },
    /// Pack one JSON frame from stdin; write the unchanged BST4 bytes to stdout.
    Pack {
        #[arg(long)]
        roots: Option<PathBuf>,
    },
    /// Build one unpublished cache part for package recovery.
    BuildPart {
        #[arg(long)]
        shard: PathBuf,
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        index: PathBuf,
    },
}
fn main() -> Result<()> {
    match Args::parse().command {
        task @ (Task::ResearchBuild { .. }
        | Task::ResearchVerify { .. }
        | Task::ResearchRead { .. }) => research_command(task)?,
        Task::Merge3 { data_dir } => {
            println!("{}", baylee_train_ops::legacy_columns::merge3(&data_dir)?);
        }
        Task::TensorIo => baylee_train_ops::corpus::tensor_io::serve(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
        )?,
        Task::CalibrationIo => json_boundary(baylee_train_ops::corpus::calibration_io::run)?,
        Task::BcCycle => json_boundary(bc_cycling::run)?,
        Task::DiagnosticMetrics => json_boundary(diagnostics::run)?,
        Task::CalibrationMetrics => json_boundary(calibration::run)?,
        Task::DiagnosticProvenance => json_boundary(diagnostic_provenance::read)?,
        Task::TrainingContract => json_boundary(training_contracts::validate)?,
        Task::Digest => {
            let value = stdin_value()?;
            println!("{}", bc_metadata::digest(&value)?);
        }
        Task::GameMetadata { data_dir, split } => {
            let value = stdin_value()?;
            let observed = serde_json::from_value::<Vec<(u64, u64)>>(value["observed"].clone())?;
            println!(
                "{}",
                bc_metadata::game_metadata(&data_dir, &value["meta"], &split, &observed)?
            );
        }
        Task::RejectOverlap => {
            let value = stdin_value()?;
            bc_metadata::reject_overlap(&value["left"], &value["right"])?;
            println!("{}", json!({"disjoint":true}));
        }
        Task::Build {
            data_dir,
            split,
            shard,
        } => println!(
            "{}",
            serde_json::to_string(&cache::build(&data_dir, &split, &shard)?)?
        ),
        Task::Validate {
            data_dir,
            policy_gradient,
            metadata_stdin,
        } => {
            let meta = if let Some(root) = &data_dir
                && !metadata_stdin
            {
                files::read_json(&root.join("meta.json"))?
            } else {
                let mut bytes = Vec::new();
                std::io::stdin().read_to_end(&mut bytes)?;
                serde_json::from_slice(&bytes)?
            };
            if policy_gradient {
                contracts::reject_policy_gradient(&meta)?;
            } else {
                contracts::validate(
                    &data_dir.context("--data-dir required for companion validation")?,
                    &meta,
                )?;
            }
            println!("{}", json!({"validated":true}));
        }
        Task::Pack { roots } => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            let payload: Value = serde_json::from_slice(&bytes)?;
            let roots = roots
                .map(|p| {
                    files::read_json(&p).and_then(|v| {
                        Ok(serde_json::from_value::<BTreeMap<String, u32>>(
                            v["roots"].clone(),
                        )?)
                    })
                })
                .transpose()?
                .unwrap_or_default();
            let (step, roots) = if let Some(step) = payload.get("step") {
                (
                    step,
                    serde_json::from_value::<BTreeMap<String, u32>>(payload["roots"].clone())?,
                )
            } else {
                (&payload, roots)
            };
            std::io::stdout().write_all(&packing::pack_step(step, &roots)?)?;
        }
        Task::BuildPart { shard, data, index } => println!(
            "{}",
            json!({"records":cache::build_part(&shard,&data,&index)?})
        ),
    }
    Ok(())
}
fn research_command(task: Task) -> Result<()> {
    match task {
        Task::ResearchBuild {
            data_dir,
            out,
            expected_manifest_sha256,
        } => println!(
            "{}",
            baylee_train_ops::research_cache::build(&data_dir, &out, &expected_manifest_sha256)?
        ),
        Task::ResearchVerify {
            cache_dir,
            expected_manifest_sha256,
        } => println!(
            "{}",
            baylee_train_ops::research_cache::verify(&cache_dir, &expected_manifest_sha256)?
        ),
        Task::ResearchRead {
            cache_dir,
            expected_manifest_sha256,
            row,
        } => {
            let mut reader = baylee_train_ops::research_cache::Reader::open(
                &cache_dir,
                &expected_manifest_sha256,
            )?;
            std::io::stdout().write_all(&reader.read_raw(row)?)?;
            std::io::stdout().write_all(b"\n")?;
        }
        _ => unreachable!("research dispatch accepts only explicit research commands"),
    }
    Ok(())
}
fn stdin_value() -> Result<Value> {
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
fn json_boundary(run: fn(&Value) -> Result<Value>) -> Result<()> {
    println!("{}", run(&stdin_value()?)?);
    Ok(())
}
