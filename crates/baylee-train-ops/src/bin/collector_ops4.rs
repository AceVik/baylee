//! Explicit native collector checks and historical one-shot lifecycle commands.
#[cfg(target_os = "linux")]
mod cli {
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
        RepairLedger {
            #[arg(long)]
            state: PathBuf,
            #[arg(long)]
            backup: PathBuf,
            #[arg(long)]
            lock: PathBuf,
            #[arg(long)]
            out: PathBuf,
            #[arg(long)]
            apply: bool,
        },
        Arm {
            #[arg(long)]
            root: PathBuf,
            #[arg(long)]
            pid: u32,
            #[arg(long)]
            thread: String,
            #[arg(long)]
            watch_module: PathBuf,
        },
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
    pub fn run() -> Result<()> {
        match Cli::parse().command {
            Mode::RepairLedger {
                state,
                backup,
                lock,
                out,
                apply,
            } => println!(
                "{}",
                collector::repair::run(&state, &backup, &lock, &out, apply)?
            ),
            Mode::Arm {
                root,
                pid,
                thread,
                watch_module,
            } => println!("{}", collector::arm(&root, pid, &thread, &watch_module)?),
            Mode::Check { root } => println!("{}", collector::monitor::check(&root)?),
            Mode::VerifyProbe { config } => {
                let config = baylee_train_ops::files::read_json(&config)?;
                anyhow::ensure!(
                    config["clean_probe_root"].is_string(),
                    "clean probe root required"
                );
                println!(
                    "{}",
                    if collector::validate_clean_probe(&config)? {
                        "verified"
                    } else {
                        "pending"
                    }
                );
            }
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
}
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    cli::run()
}
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("This process supervisor requires Linux")
}
