//! Native bounded planner/report for CPU `PyTorch` batch shape measurements.
#[cfg(target_os = "linux")]
mod cli {
    use anyhow::{Context, Result};
    use clap::Parser;
    use std::{io::Read, path::PathBuf};
    #[derive(Parser)]
    struct Args {
        #[arg(long)]
        job: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        python: Option<PathBuf>,
        #[arg(long, default_value_t = 48)]
        samples: u64,
        #[arg(long, default_value_t = 512)]
        budget_mib: u64,
        #[arg(long, default_value_t = 900.0)]
        timeout_seconds: f64,
        #[arg(long, hide = true)]
        tensor_boundary: bool,
    }
    pub fn run() -> Result<()> {
        let args = Args::parse();
        if args.tensor_boundary {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            println!(
                "{}",
                baylee_train_ops::batch_audit::boundary(&serde_json::from_str(&input)?)?
            );
        } else {
            let report = baylee_train_ops::batch_audit::cli::run(
                &args.job.context("--job required")?,
                &args.out.context("--out required")?,
                &args.python.context("--python Torch interpreter required")?,
                args.samples,
                args.budget_mib,
                args.timeout_seconds,
            )?;
            println!(
                "{}",
                serde_json::json!({"job":report["job"],"samples":report["samples"],"measured":report["measured"],"skipped":report["skipped"]})
            );
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
    anyhow::bail!("bounded tensor audit requires Linux")
}
