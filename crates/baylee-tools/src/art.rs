//! Original art generation and the pinned CC0 orchestra preparation tools.

mod dsp;
mod glb;
mod lantern;
mod memorial;
mod originality;
mod prepare;
mod samples;
mod selection;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "art",
    about = "Original lantern geometry and the pinned CC0 orchestra bank"
)]
struct Cli {
    /// Override the repository root supplied by the launcher.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Build the original lantern as portable glTF, entirely in Rust.
    Lantern(lantern::Args),
    /// Check or rebuild the pinned orchestra bank (offline by default).
    Prepare(prepare::Args),
    /// Reproduce the memorial performance or verify/rebuild its stereo bank.
    Memorial(memorial::Args),
    /// Check all 75 melodies against the listed interval signatures.
    Originality,
}

/// Execute art tools from any working directory using the launcher's root.
///
/// # Errors
/// Returns an error for invalid arguments, failed parity checks, or I/O.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let cli =
        match Cli::try_parse_from(std::iter::once("art").chain(args.iter().map(String::as_str))) {
            Ok(cli) => cli,
            Err(error)
                if matches!(
                    error.kind(),
                    clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
                ) =>
            {
                error.print()?;
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
    let root = cli.root.as_deref().unwrap_or(root);
    match cli.command {
        Command::Lantern(options) => lantern::run(root, &options),
        Command::Prepare(options) => prepare::run(root, &options),
        Command::Memorial(options) => memorial::run(root, &options),
        Command::Originality => originality::run(root),
    }
}

// Same-directory temporary files make publishing atomic per file, without a
// Python helper or a runtime tempfile dependency. IDs follow the workspace v7 rule.
fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".baylee-art-{}.tmp", uuid::Uuid::now_v7()));
    let outcome = (|| {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path).with_context(|| format!("publish {}", path.display()))
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    outcome
}
