//! `baylee-seat ranked`: an AI account that plays Ranked
//! (`baylee_seat::ranked::cli`, `docs/llm-seat.md` §"The ranked runner"):
//! the runner's own flags, and this program's pilots, the house at a fair
//! level or a language model on its chain of access paths.

use super::ranked_llm::{LlmPilot, Setup};
use baylee_client_core::llmseat::keys::KeyStore;
use baylee_seat::ranked::cli::{self, RankedArgs};
use baylee_seat::ranked::{HousePilot, Pilot};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(clap::Args)]
pub struct RankedCommand {
    #[command(flatten)]
    ranked: RankedArgs,
    /// The house's level (`novice` … `expert`); never `seer`. Central's
    /// settings for the account must name it (`house:<level>`).
    #[arg(long, default_value = "expert", conflicts_with = "profile")]
    level: String,
    /// Or a language model: the settings file's profile the account plays,
    /// its `fallback` the chain of access paths to the same model and
    /// effort (`docs/llm-seat.md` §"The ranked runner"). Central's settings
    /// for the account must name it (`model:<profile>`).
    #[arg(long)]
    profile: Option<String>,
    /// The settings file the profile is read from [default:
    /// <state>/llm-seat.json].
    #[arg(long, requires = "profile")]
    config: Option<PathBuf>,
    /// The spend book every path's game is counted in [default:
    /// <state>/llm-spend.json].
    #[arg(long, requires = "profile")]
    ledger: Option<PathBuf>,
}

/// Builds the pilot and runs the runner with it.
pub async fn run(command: &RankedCommand, keys: Arc<dyn KeyStore>) -> anyhow::Result<()> {
    let args = &command.ranked;
    let pilot: Arc<dyn Pilot> = match &command.profile {
        Some(profile) => Arc::new(LlmPilot::new(
            &Setup {
                profile: profile.clone(),
                config: args.beneath(command.config.as_ref(), "llm-seat.json"),
                ledger: args.beneath(command.ledger.as_ref(), "llm-spend.json"),
                transcripts: args.transcripts_dir(),
                house_level: args.fallback_level.clone(),
            },
            keys,
        )?),
        None => Arc::new(HousePilot::new(&command.level)?),
    };
    cli::run(args, pilot).await
}
