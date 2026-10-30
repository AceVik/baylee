//! Measure exact public structural-summary aliases in a converted dataset.
use anyhow::Result;
use baylee_train_ops::{corpus::representation, files};
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    data: PathBuf,
    #[arg(long)]
    catalog: PathBuf,
    #[arg(long)]
    out: PathBuf,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let mut result = representation::audit(&a.data, &a.catalog)?;
    files::atomic_json(&a.out, &result)?;
    if let Some(m) = result.as_object_mut() {
        m.remove("groups");
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
