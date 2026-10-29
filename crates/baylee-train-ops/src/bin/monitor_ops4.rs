//! One-shot operational observer; a user timer controls its cadence.
use anyhow::Result;
#[cfg(target_os = "linux")]
use clap::Parser;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
#[derive(Parser)]
struct Args {
    #[arg(long)]
    root: PathBuf,
    /// Compatibility with existing one-shot timer invocation.
    #[arg(long)]
    check: bool,
}
#[cfg(target_os = "linux")]
fn main() -> Result<()> {
    let args = Args::parse();
    println!("{}", baylee_train_ops::monitor::check(&args.root)?);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() -> Result<()> {
    anyhow::bail!("operational monitoring requires Linux")
}
