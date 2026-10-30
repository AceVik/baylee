//! Explicit research-only tensor service and native input gate.
use anyhow::{Result, ensure};
use baylee_train_ops::research_tensor::{self, Session, admission};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Task,
}
#[derive(Subcommand)]
enum Task {
    /// Publish dedicated input admission; numerical replay is still required before updates.
    Admit {
        #[arg(long)]
        pins: PathBuf,
        #[arg(long)]
        expected_pins_sha256: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Revalidate all source pins and the immutable dedicated admission receipt.
    Verify {
        #[arg(long)]
        pins: PathBuf,
        #[arg(long)]
        expected_pins_sha256: String,
        #[arg(long)]
        admission: PathBuf,
    },
    /// Long-lived Rust tensor reader. Explicit diagnostics never imply admission.
    Serve {
        #[arg(long)]
        cache_dir: PathBuf,
        #[arg(long)]
        expected_conversion_manifest_sha256: String,
        #[arg(long)]
        expected_cache_manifest_sha256: String,
        #[arg(long)]
        diagnostic_unadmitted: bool,
        #[arg(long,requires_all=["expected_pins_sha256","admission"])]
        pins: Option<PathBuf>,
        #[arg(long,requires_all=["pins","admission"])]
        expected_pins_sha256: Option<String>,
        #[arg(long,requires_all=["pins","expected_pins_sha256"])]
        admission: Option<PathBuf>,
    },
}
fn main() -> Result<()> {
    match Args::parse().command {
        Task::Admit {
            pins,
            expected_pins_sha256,
            out,
        } => println!("{}", admission::admit(&pins, &expected_pins_sha256, &out)?),
        Task::Verify {
            pins,
            expected_pins_sha256,
            admission: receipt,
        } => println!(
            "{}",
            admission::verify(&pins, &expected_pins_sha256, &receipt)?
        ),
        Task::Serve {
            cache_dir,
            expected_conversion_manifest_sha256,
            expected_cache_manifest_sha256,
            diagnostic_unadmitted,
            pins,
            expected_pins_sha256,
            admission: receipt,
        } => {
            let session = if let (Some(pins), Some(expected), Some(receipt)) =
                (pins, expected_pins_sha256, receipt)
            {
                ensure!(
                    !diagnostic_unadmitted,
                    "cannot combine diagnostic and admitted modes"
                );
                let session = admission::open(&pins, &expected, &receipt)?;
                ensure!(
                    session.metadata()["cache_dir"] == serde_json::json!(cache_dir.canonicalize()?)
                        && session.metadata()["conversion_manifest_sha256"]
                            == expected_conversion_manifest_sha256
                        && session.metadata()["cache_manifest_sha256"]
                            == expected_cache_manifest_sha256,
                    "serve pins conflict with admission"
                );
                session
            } else {
                ensure!(
                    diagnostic_unadmitted,
                    "serve requires admission pins or explicit --diagnostic-unadmitted"
                );
                Session::open(
                    &cache_dir,
                    &expected_conversion_manifest_sha256,
                    &expected_cache_manifest_sha256,
                )?
            };
            research_tensor::serve(session, std::io::stdin().lock(), std::io::stdout().lock())?;
        }
    }
    Ok(())
}
