//! Pinned local gateway/seat supervisor; starts only an explicit manifest.
#[cfg(target_os = "linux")]
mod cli {
    use anyhow::Result;
    use clap::Parser;
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    #[derive(Parser)]
    struct Args {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        check: bool,
    }
    pub fn run() -> Result<()> {
        let args = Args::parse();
        if args.check {
            baylee_train_ops::supervisor::check(&args.manifest)?;
            println!("Pinned local profile verified; nothing started");
            return Ok(());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::clone(&stop);
        ctrlc::set_handler(move || handler.store(true, Ordering::Relaxed))?;
        baylee_train_ops::supervisor::run(&args.manifest, &stop)
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
