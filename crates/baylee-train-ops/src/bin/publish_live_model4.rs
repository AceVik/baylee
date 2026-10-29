//! Publish a verified development preview without changing model weights.
use anyhow::Result;
use baylee_train_ops::evaluation::publication::publish;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    experiment: PathBuf,
    #[arg(long)]
    out: PathBuf,
}
fn main() -> Result<()> {
    let a = Args::parse();
    println!(
        "{}",
        serde_json::json!({"updated":publish(&a.experiment,&a.out)?})
    );
    Ok(())
}
