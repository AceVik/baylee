//! One rating ladder: every player of a spec on one Bradley-Terry/Elo scale,
//! fitted from `eval4` summaries and `ladder4` roots, one table per format
//! (`docs/trained-ai.md` §"One rating scale"). Writes the table as JSON and as
//! markdown, and prints the markdown.
use anyhow::{Context, Result};
use baylee_train_ops::evaluation::atomic_json;
use baylee_train_ops::evaluation::rating::{Options, Spec, markdown, rate};
use baylee_train_ops::files::atomic_text;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Rates players on one Bradley-Terry/Elo scale from eval4 and ladder4 results")]
struct Args {
    /// The spec: the inputs, the players on their sides, anchors and unfair
    /// players (`baylee_train_ops::evaluation::rating::Spec`).
    #[arg(long)]
    spec: PathBuf,
    /// The table as JSON.
    #[arg(long)]
    out: PathBuf,
    /// The table as markdown; without it, `--out` with the extension `md`.
    #[arg(long)]
    markdown: Option<PathBuf>,
    /// Resamples of each bootstrap, by deal and by deck.
    #[arg(long, default_value_t = 2_000)]
    resamples: usize,
    /// The bootstrap's seed: one seed, one table.
    #[arg(long, default_value_t = 20_261_110)]
    seed: u64,
    /// A player linked to the anchors by fewer games than this is flagged thin.
    #[arg(long, default_value_t = 200.0)]
    thin_link_games: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let spec = Spec::read(&args.spec)?;
    let mut report = rate(
        &spec,
        &Options {
            resamples: args.resamples,
            seed: args.seed,
            thin_link_games: args.thin_link_games,
        },
    )?;
    report["spec"] = json!(
        args.spec
            .canonicalize()
            .with_context(|| format!("spec {}", args.spec.display()))?
    );
    atomic_json(&args.out, &report)?;
    let text = markdown(&report);
    let path = args
        .markdown
        .unwrap_or_else(|| args.out.with_extension("md"));
    atomic_text(&path, &text)?;
    print!("{text}");
    Ok(())
}
