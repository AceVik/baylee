//! Build native trajectory caches and validate teacher-only dataset provenance.
use anyhow::{Context, Result};
use baylee_train_ops::{
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
