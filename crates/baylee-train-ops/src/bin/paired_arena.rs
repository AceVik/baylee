//! Historical arena comparison with paired discordances and sign-test formula.
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Arguments {
    base: PathBuf,
    #[arg(required=true,num_args=1..)]
    others: Vec<PathBuf>,
    #[arg(long)]
    out: Option<PathBuf>,
}
fn main() -> Result<()> {
    let args = Arguments::parse();
    let mut results = Vec::new();
    for other in args.others {
        let report = baylee_train_ops::legacy::arena::compare(&args.base, &other)?;
        println!("{}", serde_json::to_string(&report)?);
        results.push(report);
    }
    if let Some(out) = args.out {
        baylee_train_ops::files::atomic_json(&out, &results)
            .context("publish paired arena report")?;
    }
    Ok(())
}
