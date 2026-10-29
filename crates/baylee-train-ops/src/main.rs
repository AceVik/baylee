//! CLI boundary for native training operations.
use std::{path::PathBuf, sync::{Arc, atomic::AtomicBool}};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Native bounded training operations")]
struct Args { #[command(subcommand)] command: Task }
#[derive(Subcommand)]
enum Task {
    /// Execute one immutable attempt. Failures always require a fresh directory.
    Run {
        #[arg(long)] job: PathBuf,
        #[arg(long, default_value_t = 3600.0)] timeout: f64,
        #[arg(long, default_value_t = 48.0)] memory_gib: f64,
        #[arg(long)] input: Vec<PathBuf>,
        #[arg(last = true, required = true)] command: Vec<String>,
    },
    /// Stream SHA256 seals without allocating whole datasets.
    Hash { paths: Vec<PathBuf> },
}
fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Task::Hash { paths } => {
            for path in paths { println!("{}  {}", baylee_train_ops::files::sha256(&path)?, path.display()); }
        }
        Task::Run { job, timeout, memory_gib, input, command } => {
            #[cfg(target_os = "linux")]
            {
                let interrupted = Arc::new(AtomicBool::new(false));
                let handler = Arc::clone(&interrupted);
                ctrlc::set_handler(move || handler.store(true, std::sync::atomic::Ordering::Relaxed))?;
                let spec = baylee_train_ops::guard::Job { directory: job, cwd: std::env::current_dir()?, command, timeout_seconds: timeout, memory_gib, inputs: input };
                if !baylee_train_ops::guard::run(&spec, &interrupted)? { std::process::exit(1); }
            }
            #[cfg(not(target_os = "linux"))]
            { let _ = (job, timeout, memory_gib, input, command); anyhow::bail!("bounded process-group jobs require Linux"); }
        }
    }
    Ok(())
}
