//! Read-only streamed card-usage diagnostics for archived encoder2/3 datasets.
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    data: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    names: Option<PathBuf>,
    #[arg(long, default_value_t = 200)]
    min_offered: u64,
}
fn main() -> Result<()> {
    let args = Arguments::parse();
    let report = baylee_train_ops::legacy::house::report(
        &args.data,
        &args.out,
        args.names.as_deref(),
        args.min_offered,
    )?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
