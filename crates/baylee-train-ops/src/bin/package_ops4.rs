//! Explicit native package preparation, recovery and bounded training.
#[cfg(target_os = "linux")]
mod cli {
    use anyhow::Result;
    use baylee_train_ops::packages;
    use clap::{Parser, Subcommand};
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    #[derive(Parser)]
    #[command(about = "Immutable complete-game package operations; no implicit training recipe")]
    struct Arguments {
        #[command(subcommand)]
        command: Command,
    }
    #[derive(Subcommand)]
    enum Command {
        Prepare(packages::Producer),
        Recover(packages::Recovery),
        /// Use a new explicit recipe manifest. Never resumes the retired broad-training controller.
        Consume {
            #[arg(long)]
            manifest: PathBuf,
        },
    }
    pub fn run() -> Result<()> {
        let arguments = Arguments::parse();
        let interrupted = Arc::new(AtomicBool::new(false));
        let handler = Arc::clone(&interrupted);
        ctrlc::set_handler(move || handler.store(true, Ordering::Relaxed))?;
        match arguments.command {
            Command::Prepare(options) => packages::produce(&options, &interrupted)?,
            Command::Recover(options) => println!("{}", packages::recover(&options)?),
            Command::Consume { manifest } => {
                #[cfg(target_os = "linux")]
                packages::consume(&manifest, &interrupted)?;
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = manifest;
                    anyhow::bail!("guarded package jobs require Linux");
                }
            }
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
