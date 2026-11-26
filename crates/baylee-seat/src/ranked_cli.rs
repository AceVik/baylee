//! `baylee-seat ranked`: an AI account that plays Ranked
//! (`baylee_seat::ranked`, `docs/llm-seat.md` §"The ranked runner").

use anyhow::{Context as _, anyhow, bail};
use baylee_account::Endpoint;
use baylee_account::ranked::Format;
use baylee_client_core::llmseat::keys::KeyStore;
use baylee_client_core::say;
use baylee_seat::bridge::PlayOptions;
use baylee_seat::ranked::{
    self, Assignments as _, Credentials, Frozen, LinkOptions, MatchOptions, MatchReport, Pilot,
    Player, Queue, Settings,
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
    /// The AI account's username. Its public name must say what plays it
    /// (`KI-…` or `HOUSE-…`).
    #[arg(long)]
    username: String,
    /// A file whose first line is the account's password, readable only by
    /// this service (a systemd credential: `$CREDENTIALS_DIRECTORY/<name>`).
    /// Never the password itself.
    #[arg(
        long,
        required_unless_present = "password_key",
        conflicts_with = "password_key"
    )]
    password_file: Option<PathBuf>,
    /// Or the name the password is kept under in this machine's credential
    /// store for Central's host (`baylee-seat key set --key-env <name>
    /// --host <central>`, the password on stdin).
    #[arg(long)]
    password_key: Option<String>,
    /// The queue: `standard`, `highlander` or `commander`.
    #[arg(long, default_value = "standard")]
    format: String,
    /// The account's private deck to queue with, by its id.
    #[arg(long)]
    deck: Uuid,
    /// The house's level (`novice` … `expert`); never `seer`.
    #[arg(long, default_value = "expert")]
    level: String,
    /// Matches to play, then stop [default: play on].
    #[arg(long)]
    games: Option<u32>,
    /// A host origin whose terms the account accepts when a match is
    /// assigned there; a match on any other host is declined. Repeat for
    /// several.
    #[arg(long = "accept-host")]
    accept_hosts: Vec<String>,
    /// Give up a search after this many seconds [default: wait on].
    #[arg(long)]
    search_secs: Option<u64>,
    /// The least time between a question and the house's answer, so a
    /// person across the table can follow.
    #[arg(long, default_value_t = 1500)]
    min_think_ms: u64,
    /// Keep every match's questions and answers as JSON lines here.
    #[arg(long)]
    transcripts: Option<PathBuf>,
}

/// Signs in, plays, and signs out; a search still waiting when it stops
/// is left.
pub async fn run(
    args: &RankedArgs,
    keys: Arc<dyn KeyStore>,
    stopped: impl Future<Output = &'static str>,
) -> anyhow::Result<()> {
    let endpoint = if args.central == "live" {
        Endpoint::live()
    } else {
        Endpoint::development(&args.central)
            .map_err(|_| anyhow!("--central is `live` or a loopback http:// address"))?
    };
    let format: Format = args
        .format
        .parse()
        .map_err(|_| anyhow!("no ranked queue is called «{}»", args.format))?;
    let pilot = Pilot::house(&args.level)?;
    let credentials = match (&args.password_file, &args.password_key) {
        (Some(path), _) => Credentials::from_file(endpoint, args.username.clone(), path.clone()),
        (None, Some(name)) => {
            Credentials::from_store(endpoint, args.username.clone(), name.clone(), keys)
        }
        (None, None) => bail!("--password-file or --password-key says where the password is"),
    };
    let player = Player::sign_in(credentials, pilot.disclosure).await?;
    say!("signed in as {}", player.name());
    let alive = player.keep_alive(Duration::from_mins(10));
    let deck = Arc::new(Frozen::load(&player, args.deck, format).await?);
    let mut queue = Queue::new(
        player.clone(),
        deck,
        args.search_secs.map(Duration::from_secs),
    );
    let settings = Settings {
        games: args.games,
        play: MatchOptions {
            play: PlayOptions {
                min_think: Duration::from_millis(args.min_think_ms),
                ..PlayOptions::default()
            },
            link: LinkOptions {
                accept_hosts: args.accept_hosts.clone(),
                ..LinkOptions::default()
            },
            transcripts: args.transcripts.clone(),
            ..MatchOptions::default()
        },
        ..Settings::default()
    };
    let done = tokio::select! {
        biased;
        by = stopped => Err(anyhow!("stopped by {by}")),
        done = ranked::run(&player, &mut queue, &settings, &pilot, report) => done.map(drop),
    };
    if done.is_err() {
        queue.leave().await.context("leaving the search")?;
    }
    drop((alive, queue));
    player.sign_out().await;
    done
}

fn report(match_report: anyhow::Result<MatchReport>) {
    match match_report {
        Ok(m) => {
            let result = match m.receipt.and_then(|r| r.won().map(|won| (won, r))) {
                Some((won, r)) => format!(
                    "{} (Elo {} {:+})",
                    if won { "won" } else { "lost" },
                    r.elo,
                    r.delta
                ),
                None => "no rated result".to_owned(),
            };
            say!(
                "match {}: {result}; {} turns, {} sockets, {} renewals",
                Uuid::from_bytes(m.match_id),
                m.played.stats.turns,
                m.played.dials,
                m.renewals
            );
        }
        Err(e) => say!("a match failed: {e:#}"),
    }
}
