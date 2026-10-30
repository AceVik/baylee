//! Native experiment watcher; no trainer import or automatic process restart.
#[cfg(target_os = "linux")]
mod cli {
    use anyhow::{Context, Result};
    use clap::{Parser, Subcommand};
    use serde_json::json;
    use std::{fs, path::PathBuf};
    #[derive(Parser)]
    #[command(about = "Observe bounded experiments and wake their owning chat")]
    struct Args {
        #[command(subcommand)]
        command: Task,
    }
    #[derive(Subcommand)]
    enum Task {
        Arm {
            #[arg(long)]
            root: PathBuf,
            #[arg(long)]
            thread: String,
            #[arg(long)]
            controller_pid: Option<u32>,
            #[arg(long, required = true)]
            success_phase: Vec<String>,
            #[arg(long)]
            require_artifact: Vec<String>,
            #[arg(long, default_value_t = 30.0)]
            poll_seconds: f64,
            #[arg(long, default_value_t = 3)]
            error_polls: u64,
            #[arg(long, default_value_t = 2)]
            death_polls: u64,
        },
        #[command(alias = "run")]
        Check {
            #[arg(long)]
            root: PathBuf,
        },
    }
    pub fn run() -> Result<()> {
        match Args::parse().command {
            Task::Arm {
                root,
                thread,
                controller_pid,
                success_phase,
                require_artifact,
                poll_seconds,
                error_polls,
                death_polls,
            } => {
                let root = root.canonicalize()?;
                let pid = match controller_pid {
                    Some(pid) => pid,
                    None => fs::read_to_string(root.join("controller.pid"))?
                        .trim()
                        .parse()?,
                };
                let controller =
                    baylee_train_ops::process::identity(pid).context("controller is not live")?;
                let config = json!({"root":root,"thread":thread,"controller":controller,"success_phases":success_phase,"artifacts":require_artifact,"poll_seconds":poll_seconds,"error_polls":error_polls,"death_polls":death_polls});
                baylee_train_ops::watch::arm(&root, &config)?;
                println!("armed");
            }
            Task::Check { root } => println!("{}", baylee_train_ops::watch::check(&root)?),
        }
        Ok(())
    }
}
fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        cli::run()
    }
    #[cfg(not(target_os = "linux"))]
    anyhow::bail!("experiment process identity watches require Linux")
}
