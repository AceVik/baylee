//! baylee-catalog — CLI for the card catalog.
//!
//! ```text
//! baylee-catalog migrate                 # create the schema
//! baylee-catalog ingest                  # every card, every language
//! baylee-catalog upgrade                 # refresh an older populated catalog once
//! baylee-catalog ingest --english-only   # every card, English only
//! baylee-catalog search --query "bolt"   # check an install
//! ```
//!
//! The database URL comes from `DATABASE_URL`, so the CLI and the gateway
//! always agree on which catalog they are talking to. `--settings FILE`
//! (repeatable) reads it from the gateway's own settings files instead, the
//! last file naming it winning, so a deploy running this as root never puts
//! the URL on a command line.

use anyhow::{Context, Result};
use baylee_catalog::{Catalog, ingest, upgrade};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// Command-line arguments.
#[derive(Parser)]
#[command(about = "Card catalog: ingest and query Scryfall data")]
struct Cli {
    /// Postgres connection URL.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: Option<String>,
    /// Read `DATABASE_URL` from these `KEY=value` files instead of the
    /// environment (systemd `EnvironmentFile=` syntax; the last one wins).
    #[arg(long = "settings", value_name = "FILE")]
    settings: Vec<std::path::PathBuf>,
    #[command(subcommand)]
    command: Cmd,
}

/// What to do.
#[derive(Subcommand)]
enum Cmd {
    /// Create the schema (idempotent).
    Migrate,
    /// Refresh a populated catalog when this build needs new authoritative facts.
    Upgrade,
    /// Rebuild the search projection from what is already stored.
    Project,
    /// Download a Scryfall bulk feed and store every card in it.
    Ingest {
        /// Ingest English only instead of every language.
        #[arg(long)]
        english_only: bool,
    },
    /// Read what each type and subtype is called, out of the printings.
    ///
    /// A developer's tool, not an install step: it needs a catalog ingested
    /// in every language, and what it writes is committed as
    /// `data/type-names.tsv`. An install reads the committed file.
    MineTypes {
        /// Where to write the dictionary.
        #[arg(long, default_value = "data/type-names.tsv")]
        out: String,
    },
    /// Write the card corpus — the input a `CardIndex` is assigned from.
    ///
    /// A developer's tool like `mine-types`: it needs a catalog ingested in
    /// every language, and what it writes is fed to `cargo xtask ledger`.
    Corpus {
        /// Where to write it.
        #[arg(long, default_value = "data/card-corpus.tsv")]
        out: String,
        /// Cards to admit whatever the filter says, one oracle id per line.
        ///
        /// The hand-kept half of the corpus: cards this repo implements that
        /// Scryfall's own vocabulary drops. A missing file is an empty list.
        #[arg(long, default_value = "data/corpus-keep.tsv")]
        keep: String,
    },
    /// Search the catalog, to check an install.
    Search {
        /// What to look for.
        #[arg(long)]
        query: String,
        /// Preferred language.
        #[arg(long, default_value = "en")]
        lang: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("baylee_catalog=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    let database_url = if cli.settings.is_empty() {
        cli.database_url.clone()
    } else {
        settings_url(&cli.settings)?
    }
    .filter(|url| !url.is_empty())
    .context(
        "no DATABASE_URL: set it, pass --database-url, or name a --settings file holding it",
    )?;
    let catalog = Catalog::connect(&database_url)
        .await
        .context("connecting to the catalog")?;

    match cli.command {
        Cmd::Migrate => {
            catalog.migrate().await?;
            println!("schema is up to date");
        }
        Cmd::Upgrade => {
            catalog.migrate().await?;
            match upgrade::bulk(&catalog).await? {
                upgrade::Outcome::Empty => println!("catalog is empty; run ingest to populate it"),
                upgrade::Outcome::Current => println!("catalog ingest is current"),
                upgrade::Outcome::Refreshed(stored) => println!("refreshed {stored} printings"),
            }
        }
        Cmd::Project => {
            catalog.migrate().await?;
            catalog.project().await?;
            println!("search projection rebuilt");
        }
        Cmd::Ingest { english_only } => {
            catalog.migrate().await?;
            let feed = if english_only {
                ingest::Feed::English
            } else {
                ingest::Feed::AllLanguages
            };
            let stored = ingest::bulk(&catalog, feed).await?;
            println!(
                "stored {stored} printings ({} total)",
                catalog.count().await?
            );
        }
        Cmd::MineTypes { out } => {
            let tsv = catalog.mine_type_names().await?;
            let rows = tsv.lines().count();
            std::fs::write(&out, &tsv).with_context(|| format!("writing {out}"))?;
            println!("mined {rows} names into {out}");
        }
        Cmd::Corpus { out, keep } => {
            let kept = read_keep_list(&keep)?;
            let tsv = catalog.card_corpus(&kept).await?;
            let rows = tsv.lines().count();
            std::fs::write(&out, &tsv).with_context(|| format!("writing {out}"))?;
            println!("{rows} cards into {out} ({} kept by hand)", kept.len());
        }
        Cmd::Search { query, lang } => {
            for hit in catalog.search(&query, &lang, 20).await? {
                println!("{:<8} {:<40} {}", hit.lang, hit.name, hit.type_line);
            }
        }
    }
    Ok(())
}

/// Reads the corpus keep-list: the first column of every non-comment line.
///
/// A missing file is an empty list, not an error — the keep-list is an
/// exception register, and a checkout that has none is the ordinary case.
/// A malformed *line* is different: it would silently drop a card the repo
/// implements, so an id that is not a uuid is refused.
fn read_keep_list(path: &str) -> anyhow::Result<Vec<String>> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let mut ids = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let id = line.split('\t').next().unwrap_or_default().trim();
        anyhow::ensure!(
            id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'),
            "{path}:{}: {id:?} is not an oracle id",
            n + 1
        );
        ids.push(id.to_owned());
    }
    Ok(ids)
}

/// The last `DATABASE_URL=…` across these settings files, read in order. A
/// file that is not there is skipped, as sourcing them did; none naming the
/// URL is the caller's error.
fn settings_url(files: &[std::path::PathBuf]) -> Result<Option<String>> {
    let mut url = None;
    for file in files {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
        };
        for line in text.lines() {
            let line = line.trim_start();
            if let Some(value) = line.strip_prefix("DATABASE_URL=") {
                let value = value.trim_end();
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                    .unwrap_or(value);
                url = Some(value.to_owned());
            }
        }
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_last_settings_file_naming_the_url_wins() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (
            dir.path().join("secrets.env"),
            dir.path().join("gateway.env"),
        );
        std::fs::write(&a, "DATABASE_URL=postgres://first\nOTHER=x\n").unwrap();
        std::fs::write(&b, "# comment\nDATABASE_URL=\"postgres://second\"\n").unwrap();
        assert_eq!(
            super::settings_url(&[a.clone(), b]).unwrap().as_deref(),
            Some("postgres://second")
        );
        assert_eq!(
            super::settings_url(&[a]).unwrap().as_deref(),
            Some("postgres://first")
        );
        assert_eq!(
            super::settings_url(&[dir.path().join("absent")]).unwrap(),
            None
        );
    }
}
