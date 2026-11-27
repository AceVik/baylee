//! `baylee-seat ranked`: an AI account that plays Ranked
//! (`baylee_seat::ranked`, `docs/llm-seat.md` §"The ranked runner").

use super::ranked_llm::{LlmPilot, Setup};
use anyhow::{Context as _, anyhow, bail};
use baylee_account::Endpoint;
use baylee_account::ranked::Format;
use baylee_client_core::llmseat::keys::KeyStore;
use baylee_client_core::say;
use baylee_seat::bridge::PlayOptions;
use baylee_seat::ranked::{
    self, HousePilot, LinkOptions, MatchOptions, MatchReport, Pilot, Player, RunnerToken, Settings,
    Shelf, Stop,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(clap::Args)]
pub struct RankedArgs {
    /// Baylee Central: `live`, or a loopback development address
    /// (`http://127.0.0.1:<port>`).
    #[arg(long, default_value = "live")]
    central: String,
    /// The AI account: its id, or its whole public handle with the tag
    /// (`KI-Haus-Expert#00ab`). The runner token must list it.
    #[arg(long)]
    account: String,
    /// A file whose first line is the AI runner token (`bat_…`), readable
    /// only by this service (a systemd credential:
    /// `$CREDENTIALS_DIRECTORY/runner-token`). Never the token itself.
    #[arg(
        long,
        required_unless_present = "token_key",
        conflicts_with = "token_key"
    )]
    token_file: Option<PathBuf>,
    /// Or the name the token is kept under in this machine's credential
    /// store for Central's host (`baylee-seat key set --key-env <name>
    /// --host <central>`, the token on stdin).
    #[arg(long)]
    token_key: Option<String>,
    /// The account's state directory: `decks/`, `transcripts/` and
    /// `llm-spend.json` beneath it unless named on their own
    /// (`/var/lib/baylee-ai/<account>/`).
    #[arg(long)]
    state: Option<PathBuf>,
    /// The account's deck directory: per format `<dir>/<format>/vNNNN-….txt`,
    /// the newest that passes offered [default: <state>/decks].
    #[arg(long)]
    decks: Option<PathBuf>,
    /// Keep every match's questions and answers as JSON lines here
    /// [default: <state>/transcripts].
    #[arg(long)]
    transcripts: Option<PathBuf>,
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
    /// The house that takes a seat over from a pilot that cannot go on, and
    /// plays past the end of a language model's chain, declared.
    #[arg(long, default_value = "expert")]
    fallback_level: String,
    /// Matches to play, then stop [default: play on].
    #[arg(long)]
    games: Option<u32>,
    /// A host origin whose terms the account accepts when a match is
    /// assigned there; a match on any other host is declined. Repeat for
    /// several.
    #[arg(long = "accept-host")]
    accept_hosts: Vec<String>,
    /// The host whose legality facts the decks are checked against, fetched
    /// as a guest and kept as `<decks>/facts.json` [default: the first
    /// --accept-host].
    #[arg(long)]
    facts_from: Option<String>,
    /// The least time between a question and the answer, so a person across
    /// the table can follow.
    #[arg(long, default_value_t = 1500)]
    min_think_ms: u64,
}

/// Every ranked format: a format whose deck directory holds no deck that
/// passes offers none.
const FORMATS: [Format; 3] = [Format::Standard, Format::Highlander, Format::Commander];

/// Trades the token, plays, and signs out. A first signal withdraws the
/// offers and plays the running matches to their end; a second leaves
/// them.
pub async fn run(
    args: &RankedArgs,
    keys: Arc<dyn KeyStore>,
    stoppers: &mut super::Stoppers,
) -> anyhow::Result<()> {
    let endpoint = if args.central == "live" {
        Endpoint::live()
    } else {
        Endpoint::development(&args.central)
            .map_err(|_| anyhow!("--central is `live` or a loopback http:// address"))?
    };
    let beneath = |own: &Option<PathBuf>, name: &str| {
        own.clone()
            .or_else(|| args.state.as_ref().map(|state| state.join(name)))
    };
    let decks = beneath(&args.decks, "decks")
        .ok_or_else(|| anyhow!("--decks or --state says where the account's decks are"))?;
    let transcripts = beneath(&args.transcripts, "transcripts");
    if let Some(dir) = &transcripts {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let pilot: Arc<dyn Pilot> = match &args.profile {
        Some(profile) => Arc::new(LlmPilot::new(
            &Setup {
                profile: profile.clone(),
                config: beneath(&args.config, "llm-seat.json"),
                ledger: beneath(&args.ledger, "llm-spend.json"),
                transcripts: transcripts.clone(),
                house_level: args.fallback_level.clone(),
            },
            Arc::clone(&keys),
        )?),
        None => Arc::new(HousePilot::new(&args.level)?),
    };
    let fallback: Arc<dyn Pilot> = Arc::new(HousePilot::new(&args.fallback_level)?);
    let token = match (&args.token_file, &args.token_key) {
        (Some(path), _) => RunnerToken::from_file(path.clone()),
        (None, Some(name)) => RunnerToken::from_store(&endpoint, name.clone(), keys),
        (None, None) => bail!("--token-file or --token-key says where the runner token is"),
    };
    let account = ranked::resolve(&endpoint, &args.account).await?;
    let player = Player::open(endpoint, account, token).await?;
    say!(
        "playing {} ({}) with {}",
        player.name(),
        Uuid::from_bytes(account),
        pilot.profile()
    );
    let alive = player.keep_alive(Duration::from_mins(5));
    let mut shelf = Shelf::directory(decks, FORMATS.to_vec());
    let settings = Settings {
        games: args.games,
        facts_from: args
            .facts_from
            .clone()
            .or_else(|| args.accept_hosts.first().cloned()),
        play: MatchOptions {
            play: PlayOptions {
                min_think: Duration::from_millis(args.min_think_ms),
                ..PlayOptions::default()
            },
            link: LinkOptions {
                accept_hosts: args.accept_hosts.clone(),
                ..LinkOptions::default()
            },
            transcripts,
            ..MatchOptions::default()
        },
        ..Settings::default()
    };
    let (stop, stopping) = tokio::sync::watch::channel(Stop::Run);
    let done = {
        let runner = ranked::run(
            &player, &mut shelf, pilot, fallback, &settings, stopping, report,
        );
        tokio::pin!(runner);
        let mut asked = Stop::Run;
        loop {
            tokio::select! {
                done = &mut runner => break done,
                by = stoppers.stopped() => {
                    asked = if asked == Stop::Run { Stop::Drain } else { Stop::Leave };
                    say!(
                        "stopped by {by}: {}",
                        if asked == Stop::Drain {
                            "no more offers; the running matches play to their end \
                             (stop it again to leave them)"
                        } else {
                            "leaving the running matches"
                        }
                    );
                    let _ = stop.send(asked);
                }
            }
        }
    };
    drop(alive);
    player.sign_out().await;
    done.map(|played| say!("played {played} matches"))
}

fn report(match_report: anyhow::Result<MatchReport>) {
    match match_report {
        Ok(m) => {
            let result = match m.receipt.and_then(|r| r.won().map(|won| (won, r))) {
                Some((won, r)) => format!(
                    "{} (Elo {}{})",
                    if won { "won" } else { "lost" },
                    r.elo,
                    r.delta.map_or_default(|d| format!(" {d:+}"))
                ),
                None => "no rated result".to_owned(),
            };
            let handed = m
                .handed_over
                .map_or_default(|why| format!("; the house took over: {why}"));
            say!(
                "match {}: {result}; {} turns, {} sockets, {} renewals{handed}",
                Uuid::from_bytes(m.match_id),
                m.played.stats.turns,
                m.played.dials,
                m.renewals
            );
        }
        Err(e) => say!("a match failed: {e:#}"),
    }
}
