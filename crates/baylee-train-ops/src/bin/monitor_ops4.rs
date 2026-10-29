//! One-shot operational observer; a user timer controls its cadence.
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    root: PathBuf,
    /// Compatibility with existing one-shot timer invocation.
    #[arg(long)]
    check: bool,
}
fn main() -> Result<()> {
    let args = Args::parse();
    #[cfg(target_os = "linux")]
    println!("{}", baylee_train_ops::monitor::check(&args.root)?);
    #[cfg(not(target_os = "linux"))]
    {
        let _ = args;
        anyhow::bail!("operational monitoring requires Linux");
    }
    Ok(())
}
