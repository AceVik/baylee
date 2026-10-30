//! Collection only: official anonymized export and native dedup import, no training.
#[cfg(target_os = "linux")]
mod cli {
    use anyhow::{Result, ensure};
    use baylee_train_ops::{
        evaluation::maintenance::{Settings, collect},
        files, process,
    };
    use clap::Parser;
    use serde_json::json;
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    #[derive(Parser)]
    struct Args {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        gateway_bin: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long,default_value_t=0,value_parser=clap::value_parser!(u64).range(0..=1))]
        neural_seat: u64,
        #[arg(long)]
        once: bool,
    }
    pub fn run() -> Result<()> {
        let args = Args::parse();
        ensure!(
            args.root.is_absolute() && args.gateway_bin.is_absolute() && args.corpus.is_absolute(),
            "absolute collector paths required"
        );
        ensure!(
            std::env::var_os("DATABASE_URL").is_some_and(|v| !v.is_empty()),
            "official export environment missing"
        );
        ensure!(
            std::env::var_os("BAYLEE_AGENT_TOKEN").is_none(),
            "collection must not inherit agent credentials"
        );
        let _lock = files::lock(&args.root.join("collector.lock"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::clone(&stop);
        ctrlc::set_handler(move || handler.store(true, Ordering::Relaxed))?;
        let settings = Settings {
            experiment: args.root.clone(),
            selection: args.root.join("unused-selection"),
            gateway_bin: args.gateway_bin,
            corpus: args.corpus.clone(),
            neural_seat: args.neural_seat,
            status: args.root.join("collection-status.json"),
        };
        let mut state = json!({"phase":"watching","identity":process::identity(std::process::id()),"corpus":args.corpus,"interval_seconds":300,"importer":"native-record-import-v1","training_done":false});
        while !stop.load(Ordering::Relaxed) {
            match collect(&settings, &stop) {
                Ok(value) => {
                    state["collection"] = value;
                    state
                        .as_object_mut()
                        .expect("state object")
                        .remove("error_type");
                }
                Err(_) => {
                    state["error_type"] = json!("CollectionError");
                }
            }
            state["checked_unix"] = json!(files::unix_time()?);
            files::atomic_json(&settings.status, &state)?;
            if args.once {
                break;
            }
            let deadline = Instant::now() + Duration::from_secs(300);
            while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        state["phase"] = json!("stopped");
        files::atomic_json(&settings.status, &state)
    }
}
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    cli::run()
}
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("This process supervisor requires Linux")
}
