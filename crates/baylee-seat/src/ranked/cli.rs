//! The ranked runner's command line and process, for every program that
//! flies an AI account (`baylee-seat ranked`, the trained net's `seat4`):
//! flatten [`RankedArgs`] into the program's own arguments, build its
//! [`Pilot`], and hand both to [`run`], which does the rest (the process's
//! stop signals, logging and TLS, the credential store, the token, the
//! decks, offers and work, every match, and the house fallback).

use super::{
    HousePilot, LinkOptions, MatchOptions, MatchReport, Pilot, Player, RunnerToken, Settings,
    Shelf, Stop,
};
use crate::bridge::PlayOptions;
use crate::stop::Stoppers;
use anyhow::{Context as _, anyhow, bail};
use baylee_account::Endpoint;
use baylee_account::ranked::Format;
use baylee_client_core::say;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Everything the runner takes from its command line but the pilot: a
/// program adds its own pilot's flags beside it (`--level`, `--profile`).
#[derive(Clone, Debug, clap::Args)]
pub struct RankedArgs {
    /// Baylee Central: `live`, or a loopback development address
    /// (`http://127.0.0.1:<port>`).
    #[arg(long, default_value = "live")]
    pub central: String,
    /// The AI account: its id, or its whole public handle with the tag
    /// (`KI-Haus-Expert#00ab`). The runner token must list it.
    #[arg(long)]
    pub account: String,
    /// A file whose first line is the AI runner token (`bat_…`), readable
    /// only by this service (a systemd credential:
    /// `$CREDENTIALS_DIRECTORY/runner-token`). Never the token itself.
    #[arg(
        long,
        required_unless_present = "token_key",
        conflicts_with = "token_key"
    )]
    pub token_file: Option<PathBuf>,
    /// Or the name the token is kept under in this machine's credential
    /// store for Central's host (`baylee-seat key set --key-env <name>
    /// --host <central>`, the token on stdin).
    #[arg(long)]
    pub token_key: Option<String>,
    /// The account's state directory: `decks/`, `transcripts/` and
    /// `llm-spend.json` beneath it unless named on their own
    /// (`/var/lib/baylee-ai/<account>/`).
    #[arg(long)]
    pub state: Option<PathBuf>,
    /// The account's deck directory: per format `<dir>/<format>/vNNNN-….txt`,
    /// the newest that passes offered [default: <state>/decks].
    #[arg(long)]
    pub decks: Option<PathBuf>,
    /// Keep every match's questions and answers as JSON lines here
    /// [default: <state>/transcripts].
    #[arg(long)]
    pub transcripts: Option<PathBuf>,
    /// The house that takes a seat over from a pilot that cannot go on, and
    /// plays past the end of a language model's chain, declared.
    #[arg(long, default_value = "expert")]
    pub fallback_level: String,
    /// Matches to play, then stop [default: play on].
    #[arg(long)]
    pub games: Option<u32>,
    /// A host origin whose terms the account accepts when a match is
    /// assigned there; a match on any other host is declined. Repeat for
    /// several.
    #[arg(long = "accept-host")]
    pub accept_hosts: Vec<String>,
    /// The host whose legality facts the decks are checked against, fetched
    /// as a guest now and then and kept as `<decks>/facts.json`, best
    /// effort [default: the first --accept-host, unless --facts-file].
    #[arg(long)]
    pub facts_from: Option<String>,
    /// Or legality facts the operator keeps (`baylee-seat deck facts`'s
    /// file), read and never written; beside --facts-from, the fresher of
    /// the two is used.
    #[arg(long)]
    pub facts_file: Option<PathBuf>,
    /// The least time between a question and the answer, so a person across
    /// the table can follow.
    #[arg(long, default_value_t = 1500)]
    pub min_think_ms: u64,
}

impl RankedArgs {
    /// `own`, else `name` beneath `--state`.
    #[must_use]
    pub fn beneath(&self, own: Option<&PathBuf>, name: &str) -> Option<PathBuf> {
        own.cloned()
            .or_else(|| self.state.as_ref().map(|state| state.join(name)))
    }

    /// Where the transcripts go, if anywhere.
    #[must_use]
    pub fn transcripts_dir(&self) -> Option<PathBuf> {
        self.beneath(self.transcripts.as_ref(), "transcripts")
    }

    /// The Central named.
    ///
    /// # Errors
    /// Neither `live` nor a loopback address.
    pub fn endpoint(&self) -> anyhow::Result<Endpoint> {
        if self.central == "live" {
            Ok(Endpoint::live())
        } else {
            Endpoint::development(&self.central)
                .map_err(|_| anyhow!("--central is `live` or a loopback http:// address"))
        }
    }
}

/// Every ranked format: a format whose deck directory holds no deck that
/// passes offers none.
const FORMATS: [Format; 3] = [Format::Standard, Format::Highlander, Format::Commander];

/// What a program that plays a seat sets up first, once: logging to
/// stderr as `RUST_LOG` says, and rustls's one TLS provider. Either one
/// already set up stays as it is.
pub fn set_up_process() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();
    // One TLS provider, named: the crates that speak TLS here ask rustls for
    // a default rather than bringing one.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Flies `pilot` on the AI account `args` names until it is stopped or
/// its games are played: the stop signals armed first, the process set up
/// ([`set_up_process`]), the token traded, the decks offered, every match
/// played, and the sessions signed out. A first signal withdraws the
/// offers and plays the running matches to their end; a second leaves
/// them.
///
/// # Errors
/// A Central or a fallback level that cannot be, no deck directory, a
/// refused token, or an account that cannot be found.
pub async fn run(args: &RankedArgs, pilot: Arc<dyn Pilot>) -> anyhow::Result<()> {
    play(args, pilot, report)
        .await
        .map(|played| say!("played {played} matches"))
}

/// [`run`], each match told to `each` as it ends: how many it played.
///
/// # Errors
/// As [`run`].
pub async fn play(
    args: &RankedArgs,
    pilot: Arc<dyn Pilot>,
    each: impl FnMut(anyhow::Result<MatchReport>) + Send,
) -> anyhow::Result<u32> {
    let mut stoppers = Stoppers::arm();
    set_up_process();
    let endpoint = args.endpoint()?;
    let decks = args
        .beneath(args.decks.as_ref(), "decks")
        .ok_or_else(|| anyhow!("--decks or --state says where the account's decks are"))?;
    let transcripts = args.transcripts_dir();
    if let Some(dir) = &transcripts {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let fallback: Arc<dyn Pilot> = Arc::new(HousePilot::new(&args.fallback_level)?);
    let token = match (&args.token_file, &args.token_key) {
        (Some(path), _) => RunnerToken::from_file(path.clone()),
        // The store is opened only where the token is kept there.
        (None, Some(name)) => RunnerToken::from_store(
            &endpoint,
            name.clone(),
            crate::keys::store(&|name: &str| std::env::var(name).ok()),
        ),
        (None, None) => bail!("--token-file or --token-key says where the runner token is"),
    };
    let account = super::resolve(&endpoint, &args.account).await?;
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
        facts_from: args.facts_from.clone().or_else(|| {
            args.facts_file
                .is_none()
                .then(|| args.accept_hosts.first().cloned())
                .flatten()
        }),
        facts_file: args.facts_file.clone(),
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
        let runner = super::run(
            &player, &mut shelf, pilot, fallback, &settings, stopping, each,
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
    done
}

/// Says how a match went, in a line.
fn report(match_report: anyhow::Result<MatchReport>) {
    match match_report {
        Ok(m) => {
            let result = match m.receipt.and_then(|r| r.won().map(|won| (won, r))) {
                Some((won, r)) => format!(
                    "{} (Elo {}{}, inferred from the account's standing)",
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
