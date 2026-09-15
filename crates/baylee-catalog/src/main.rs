//! baylee-catalog — CLI for the card catalog.
//!
//! ```text
//! baylee-catalog migrate                 # create the schema
//! baylee-catalog ingest                  # every card, every language
//! baylee-catalog ingest --english-only   # every card, English only
//! baylee-catalog search --query "bolt"   # check an install
//! ```
//!
//! The database URL comes from `DATABASE_URL`, so the CLI and the gateway
//! always agree on which catalog they are talking to.

use anyhow::{Context, Result};
use baylee_catalog::{Catalog, ingest};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// Command-line arguments.
#[derive(Parser)]
#[command(about = "Card catalog: ingest and query Scryfall data")]
struct Cli {
    /// Postgres connection URL.
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    #[command(subcommand)]
    command: Cmd,
}

/// What to do.
#[derive(Subcommand)]
enum Cmd {
    /// Create the schema (idempotent).
    Migrate,
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
    let catalog = Catalog::connect(&cli.database_url)
        .await
        .context("connecting to the catalog")?;

    match cli.command {
        Cmd::Migrate => {
            catalog.migrate().await?;
            println!("schema is up to date");
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
        Cmd::Search { query, lang } => {
            for hit in catalog.search(&query, &lang, 20).await? {
                println!("{:<8} {:<40} {}", hit.lang, hit.name, hit.type_line);
            }
        }
    }
    Ok(())
}
