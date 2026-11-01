//! Stage official anonymized gateway exports; never create policy labels.
use anyhow::Result;
use baylee_train_ops::evaluation::live_import::import_records;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    export_dir: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long,default_value_t=0,value_parser=clap::value_parser!(u64).range(0..=1))]
    neural_seat: u64,
    #[arg(long, alias = "exclude")]
    exclude_sha256: Option<PathBuf>,
}
fn main() -> Result<()> {
    let a = Args::parse();
    if let Ok(value) = import_records(
        &a.export_dir,
        &a.out,
        a.neural_seat,
        a.exclude_sha256.as_deref(),
    ) {
        println!("{}", serde_json::to_string(&value)?);
        Ok(())
    } else {
        eprintln!("live record import refused: invalid export, exclusion, or existing store");
        std::process::exit(1);
    }
}
