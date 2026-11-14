//! Offline preparation, independent of the engine being repaired/refactored.
//! Reference-case success is not evidence that a game or service implements it.
use anyhow::{Result, ensure};
use clap::{Args, Subcommand};
use std::path::PathBuf;

mod cases;
mod execution;
mod observations;
mod pacing;
mod recipes;

#[cfg(test)]
mod tests;

/// Read-only preparation commands; no subprocess, database or network access.
#[derive(Args)]
pub(crate) struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate the reference corpus and its manually specified expectations.
    Check,
    /// Export versioned inputs/expectations for a future production test adapter.
    Fixtures,
    /// Compare all adapter observations; omitted, duplicate or extra cases fail.
    Compare {
        /// JSON produced by an adapter exercising actual implementation code.
        observations: PathBuf,
    },
    /// Independent-win toy model, not measured balance or an acquisition simulation.
    Pacing {
        #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u8).range(1..=32))]
        wins: u8,
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=32))]
        defeats: u8,
        /// Assumed probability per duel, in thousandths; no draw correlation model.
        #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u16).range(0..=1000))]
        win_per_mille: u16,
    },
}

/// Execute preparation without changing the repository or starting a service.
pub(crate) fn run(options: Options) -> Result<i32> {
    let suite = cases::suite();
    suite.check()?;
    let report = match options.command {
        Command::Fixtures => serde_json::to_value(&suite)?,
        Command::Check => serde_json::json!({
            "scope": "reference_contract_only",
            "contract_revision": cases::CONTRACT_REVISION,
            "fusion_cases": suite.fusions.len(),
            "resale_cases": suite.resales.len(),
            "execution_cases": suite.executions.len(),
            "production_adapter_connected": false,
            "engine_execution_tested": false,
        }),
        Command::Compare { observations: path } => {
            let count = observations::compare(&suite, &observations::read(&path)?)?;
            serde_json::json!({
                "scope": "caller_supplied_adapter_observations",
                "contract_revision": cases::CONTRACT_REVISION,
                "matching_cases": count,
                "artifact_contents_verified": false,
                "engine_execution_tested": false,
                "note": "Passing observations do not authenticate their producer or prove binary contents.",
            })
        }
        Command::Pacing {
            wins,
            defeats,
            win_per_mille,
        } => serde_json::to_value(pacing::calculate(wins, defeats, win_per_mille)?)?,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(0)
}

fn bounded_label(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 80
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn schema(version: u32, revision: u32) -> Result<()> {
    ensure!(
        version == 1 && revision == cases::CONTRACT_REVISION,
        "unsupported observation contract"
    );
    Ok(())
}
