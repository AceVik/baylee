//! Maintain local preview selection and import anonymized completed games.
use anyhow::Result;
use baylee_train_ops::evaluation::maintenance::{Settings, run};
use clap::Parser;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Parser)]
struct Args {
    #[arg(long)]
    experiment: PathBuf,
    #[arg(long)]
    selection: PathBuf,
    #[arg(long)]
    gateway_bin: PathBuf,
    #[arg(long)]
    corpus: PathBuf,
    #[arg(long,default_value_t=0,value_parser=clap::value_parser!(u64).range(0..=1))]
    neural_seat: u64,
    #[arg(long)]
    status: PathBuf,
    #[arg(long)]
    once: bool,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let stopped = Arc::new(AtomicBool::new(false));
    let handler = Arc::clone(&stopped);
    ctrlc::set_handler(move || handler.store(true, Ordering::Relaxed))?;
    run(
        &Settings {
            experiment: a.experiment,
            selection: a.selection,
            gateway_bin: a.gateway_bin,
            corpus: a.corpus,
            neural_seat: a.neural_seat,
            status: a.status,
        },
        &stopped,
        a.once,
    )
}
