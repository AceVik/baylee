//! Local render recordings: loopback capture, GIF/WebP, sheets and diagnostics.
mod cli;
mod control;
mod data;
mod render;
mod sheet;
mod timing;
mod webp;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use serde_json::json;
use std::io::Write;
use std::path::Path;

/// Run the recording CLI. Recording paths are relative to the caller's working
/// directory; no repository assets, renderer, Python or external encoder needed.
pub fn run(_root: &Path, args: &[String]) -> Result<()> {
    let cli = match Cli::try_parse_from(
        std::iter::once("recording".to_string()).chain(args.iter().cloned()),
    ) {
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
    let result = match cli.command {
        Command::Record {
            port,
            duration_ms,
            fps,
            preview,
        } => control::record(port, duration_ms, fps, &preview)?,
        Command::Encode {
            directory,
            allow_incomplete,
            preview,
        } => render::encode(&directory, &preview, allow_incomplete)?,
        Command::Status { port } => control::request(port, "status", None)?,
        Command::Stop { port, cancel } => {
            control::request(port, "stop", Some(&json!({"cancel":cancel})))?
        }
    };
    let mut output = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &result)?;
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod tests;
