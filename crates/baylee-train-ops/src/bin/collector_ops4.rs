//! Explicit native collector checks and historical one-shot lifecycle commands.
use anyhow::Result;
use baylee_train_ops::collector;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Mode,
}
#[derive(Subcommand)]
enum Mode {
    Check {
        #[arg(long)]
        root: PathBuf,
    },
    VerifyProbe {
        #[arg(long)]
        config: PathBuf,
    },
    Finish {
        #[arg(long)]
        ops: PathBuf,
    },
    ResumeLane9 {
        #[arg(long)]
        ops: PathBuf,
    },
    ProbePreflight {
        plan: PathBuf,
    },
    Launch {
        plan: PathBuf,
    },
}
fn main() -> Result<()> {
    match Cli::parse().command {
        Mode::Check { root } => println!("{}", collector::monitor::check(&root)?),
        Mode::VerifyProbe { config } => println!(
            "{}",
            if collector::validate_clean_probe(&baylee_train_ops::files::read_json(&config)?)? {
                "verified"
            } else {
                "pending"
            }
        ),
        Mode::Finish { ops } => println!("{}", collector::finish(&ops)?),
        Mode::ResumeLane9 { ops } => println!("{}", collector::resume(&ops)?),
        Mode::ProbePreflight { plan } => {
            collector::launch::probe_preflight(&plan)?;
            println!("Pinned inputs and reset time verified; nothing started");
        }
        Mode::Launch { plan } => collector::launch::run(&plan)?,
    }
    Ok(())
}
