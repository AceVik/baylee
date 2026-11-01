//! Landing-page catalog generation, content stamps and private Impressum blobs.

mod bulk;
mod card_status;
mod catalog;
mod checks;
mod html;
mod impressum;
mod serve;
mod stamp;

#[cfg(test)]
mod tests_card;
#[cfg(test)]
mod tests_impressum;
#[cfg(test)]
mod tests_page;
#[cfg(test)]
mod tests_parity;
#[cfg(test)]
mod tests_worker;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "site", about = "Landing-page maintenance and privacy checks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate the catalog from a single committed revision.
    CardStatus(card_status::Args),
    /// Fail unless `web/site/data` was measured at an ancestor of REV whose
    /// pool REV did not change (what a release tag must carry).
    CardStatusFresh {
        #[arg(long, default_value = "HEAD")]
        rev: String,
    },
    /// Update content hashes (CSS first, then HTML).
    Stamp {
        /// Report stale stamps without writing files.
        #[arg(long)]
        check: bool,
    },
    /// Encode private JSON into an outside-repository Impressum blob.
    ImpressumBlob { json: PathBuf, out: Option<PathBuf> },
    /// Preview public web files on a loopback address.
    Serve {
        #[arg(long, default_value_t = 8000)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
    /// Check committed catalog, accessibility, privacy and content stamps offline.
    Check,
}

/// Run a `site` subcommand; `args` excludes the outer executable and `site`.
///
/// # Errors
/// Returns an error for invalid arguments, stale stamps, violated guards or I/O.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let cli = Cli::try_parse_from(std::iter::once("site".to_owned()).chain(args.iter().cloned()))?;
    match cli.command {
        Command::CardStatus(args) => card_status::run(root, args),
        Command::CardStatusFresh { rev } => card_status::fresh(root, &rev),
        Command::Stamp { check } => stamp::run(root, check),
        Command::ImpressumBlob { json, out } => {
            let out = out.map_or_else(impressum::default_out, Ok)?;
            let size = impressum::write_blob(root, &json, &out)?;
            // Never print input/output paths or field values: filenames may contain PII too.
            println!("wrote Impressum blob ({size} bytes, 8 fields)");
            Ok(())
        }
        Command::Check => checks::run(root),
        Command::Serve { port, bind } => serve::run(root, port, &bind),
    }
}

#[cfg(test)]
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
