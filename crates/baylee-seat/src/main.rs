//! `baylee-seat`: a mind at a table, as an ordinary socket player.
//!
//! ```text
//! baylee-seat join <room> --mind house|scripted [--deck <file> | --acceptance <name>]
//! ```
//!
//! Signs in as a guest under the name its mind discloses (`HOUSE-house`,
//! `TEST-scripted`; a language model's is `LLM-…`), stores the deck,
//! takes a free chair in the room, says ready, waits for the host to start,
//! and plays the game to its end. The gateway is `--gateway`, else
//! `BAYLEE_GATEWAY`, else the local default; a closed beta's key is
//! `--invite-key` or `BAYLEE_INVITE_KEY`, a locked room's password
//! `--password` or `BAYLEE_ROOM_PASSWORD`. `RUST_LOG=info` says what the
//! bridge is doing; `--transcripts <dir>` keeps every question and answer as
//! JSON lines.

use anyhow::{Context as _, bail};
use baylee_ai::AIProfile;
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{Chair, GuestSignIn, Lobby, Session, seat_name};
use baylee_seat::seat::{BLITZ_SECS, Outcome};
use baylee_seat::{BridgeConfig, HouseMind, Mind, ScriptedMind, SeatCore, Transcript};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The gateway a bridge dials when told nothing.
const LOCAL_GATEWAY: &str = "http://127.0.0.1:28766";

#[derive(Parser)]
#[command(
    name = "baylee-seat",
    about = "A mind at a Baylee table, as an ordinary socket player"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Take a free chair in a room and play its game.
    Join(Join),
}

#[derive(clap::Args)]
struct Join {
    /// The room's id, as the lobby lists it.
    room: String,
    /// The gateway's address [default: `BAYLEE_GATEWAY`, else the local one].
    #[arg(long)]
    gateway: Option<String>,
    /// What decides.
    #[arg(long, value_enum, default_value_t = MindKind::House)]
    mind: MindKind,
    /// A deck file in any format a player can export.
    #[arg(long, conflicts_with = "acceptance")]
    deck: Option<PathBuf>,
    /// One of the acceptance decks, by name (the default when no file is
    /// given: Allytifact).
    #[arg(long)]
    acceptance: Option<String>,
    /// The name after the mind's prefix (`HOUSE-`, `TEST-`) [default: the
    /// mind's kind].
    #[arg(long)]
    name: Option<String>,
    /// The house's level: the house mind's own, and the fallback's.
    #[arg(long, default_value = "steady")]
    level: String,
    /// The room's password [default: `BAYLEE_ROOM_PASSWORD`].
    #[arg(long)]
    password: Option<String>,
    /// A closed beta's key [default: `BAYLEE_INVITE_KEY`].
    #[arg(long)]
    invite_key: Option<String>,
    /// The longest one answer may take, whatever the table's clock allows.
    #[arg(long, default_value_t = 60)]
    think_secs: u64,
    /// The least time between a woken question and its answer.
    #[arg(long, default_value_t = 1500)]
    min_think_ms: u64,
    /// A directory for the game's transcript, one JSON line per event.
    #[arg(long)]
    transcripts: Option<PathBuf>,
    /// Sit at a table that gives a question 30 seconds or fewer.
    #[arg(long)]
    allow_blitz: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum MindKind {
    /// The house heuristic, on what the seat sees.
    House,
    /// The least answer to every question: a seat that plays nothing.
    Scripted,
}

impl MindKind {
    /// The mind this choice plays with.
    fn mind(self, profile: AIProfile) -> Arc<dyn Mind> {
        match self {
            Self::House => Arc::new(HouseMind::new(profile)),
            Self::Scripted => Arc::new(ScriptedMind::idle()),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::House => "house",
            Self::Scripted => "scripted",
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    // One TLS provider, named: the crates that speak TLS here ask rustls for
    // a default rather than bringing one.
    let _ = rustls::crypto::ring::default_provider().install_default();
    match Cli::parse().command {
        Command::Join(join) => {
            let seated = sit_down(&join).await?;
            let played = play_out(&join, seated).await?;
            report(&played);
            Ok(())
        }
    }
}

/// A chair taken, and what the seat plays with.
struct Seated {
    lobby: Lobby,
    session: Session,
    chair: Chair,
    deck: Deck,
    profile: AIProfile,
    mind: Arc<dyn Mind>,
}

/// The name a chair played by `mind` signs in under: the prefix of what
/// the mind says it is, then `--name` or the mind's kind.
fn display_name(name: Option<&str>, kind: MindKind, mind: &dyn Mind) -> anyhow::Result<String> {
    seat_name(mind.disclosure(), name.unwrap_or(kind.label()))
}

/// The seat that plays for `mind`, held at the table to the name
/// [`display_name`] gave the chair: both are what the mind says it is.
fn seat_core(config: BridgeConfig, deck: &Deck, mind: &dyn Mind) -> SeatCore {
    SeatCore::new(config, deck.list.clone(), mind.disclosure())
}

/// Signs in, checks the room, takes a chair, says ready and waits for the
/// host to start.
async fn sit_down(join: &Join) -> anyhow::Result<Seated> {
    let profile = AIProfile::named(&join.level)
        .with_context(|| format!("no house level called «{}»", join.level))?;
    let mind = join.mind.mind(profile);
    let display_name = display_name(join.name.as_deref(), join.mind, &*mind)?;
    let deck = match (&join.deck, &join.acceptance) {
        (Some(path), _) => Deck::from_file(path)?,
        (None, name) => Deck::acceptance(name.as_deref().unwrap_or("Allytifact"))?,
    };
    let password = join
        .password
        .clone()
        .or_else(|| std::env::var("BAYLEE_ROOM_PASSWORD").ok());
    let invite_key = join
        .invite_key
        .clone()
        .or_else(|| std::env::var("BAYLEE_INVITE_KEY").ok());
    let gateway = join
        .gateway
        .clone()
        .or_else(|| std::env::var("BAYLEE_GATEWAY").ok())
        .unwrap_or_else(|| LOCAL_GATEWAY.to_string());

    let lobby = Lobby::new(&gateway);
    let session = lobby
        .guest(&GuestSignIn {
            display_name: display_name.clone(),
            invite_key,
        })
        .await?;
    let Some(room) = lobby.room(&session, &join.room).await? else {
        bail!("no room {} on {gateway}", join.room);
    };
    if !room.waiting() {
        bail!(
            "the room {} is {}, not waiting for players",
            room.id,
            room.state
        );
    }
    if !room.has_a_free_chair() {
        bail!("the room {} has no free chair", room.id);
    }
    if room.locked && password.is_none() {
        bail!("the room {} is locked: give its --password", room.id);
    }
    if let Some(secs) = room.clock.decide_secs.filter(|s| *s <= BLITZ_SECS)
        && !join.allow_blitz
    {
        bail!(
            "the room {} gives {secs} s a question, too few for a mind that thinks \
             (sit anyway with --allow-blitz)",
            room.id
        );
    }
    let deck_id = lobby.upload(&session, &deck).await?;
    let chair = lobby
        .join(&session, &room.id, &deck_id, password.as_deref())
        .await?;
    lobby.ready(&session, &room.id).await?;
    println!(
        "«{display_name}» sits in chair {} of room {} with {}; waiting for the host to start",
        chair.seat, room.id, deck.name
    );
    lobby
        .wait_for_start(&session, &room.id, Duration::from_secs(1))
        .await?;
    println!("the game is on");
    Ok(Seated {
        lobby,
        session,
        chair,
        deck,
        profile,
        mind,
    })
}

/// Plays the seated chair's game to its end.
async fn play_out(join: &Join, seated: Seated) -> anyhow::Result<bridge::Played> {
    let config = BridgeConfig {
        think: Duration::from_secs(join.think_secs),
        allow_blitz: join.allow_blitz,
        house: seated.profile,
        ..BridgeConfig::default()
    };
    let core = seat_core(config, &seated.deck, &*seated.mind);
    let mut transcript = match &join.transcripts {
        Some(dir) => {
            let path = dir.join(format!(
                "{}-seat{}.jsonl",
                seated.chair.game_id, seated.chair.seat
            ));
            Transcript::file(&path).with_context(|| format!("open {}", path.display()))?
        }
        None => Transcript::none(),
    };
    let options = PlayOptions {
        min_think: Duration::from_millis(join.min_think_ms),
        ..PlayOptions::default()
    };
    let mut link = SeatLink::new(seated.lobby, seated.chair, Some(seated.session));
    let played = bridge::play(&mut link, core, seated.mind, &mut transcript, &options).await?;
    transcript.write_value(&serde_json::json!({ "summary": played.stats }));
    transcript.flush();
    Ok(played)
}

/// The line a game ends with.
fn report(played: &bridge::Played) {
    let stats = &played.stats;
    let outcome = match stats.outcome {
        Some(Outcome::Won) => "won",
        Some(Outcome::Lost) => "lost",
        Some(Outcome::Draw) => "drew",
        None if played.result.is_none() => "the room closed without a result",
        None => "ended",
    };
    println!(
        "{outcome} after {} turns: {} questions, {} answered by standing orders, {} wakes \
         and {} payment steps; the mind answered {}, the house {}, the least answer {}; \
         {} fallbacks, {} refused, {} ms of model time, {} sockets",
        stats.turns,
        stats.questions,
        stats.standing.total(),
        stats.wakes,
        stats.continuations,
        stats.answered.mind,
        stats.answered.house,
        stats.answered.least,
        stats.fallbacks.total(),
        stats.refused_by_referee + stats.refused_by_table,
        stats.model_ms,
        played.dials,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_seat::Disclosure;

    /// A chair's name tells the truth about its mind (the owner's rule):
    /// the house signs in as the house and a script as a test, never as a
    /// language model, and the seat holds the table to that same name.
    #[test]
    fn every_mind_sits_under_the_name_of_what_it_is() {
        let deck = Deck::acceptance("Victory").unwrap();
        for &kind in MindKind::value_variants() {
            let mind = kind.mind(AIProfile::default());
            let (named, prefix) = match kind {
                MindKind::House => ("HOUSE-house", "HOUSE-"),
                MindKind::Scripted => ("TEST-scripted", "TEST-"),
            };
            assert_eq!(display_name(None, kind, &*mind).unwrap(), named);
            let chosen = display_name(Some("x1"), kind, &*mind).unwrap();
            assert_eq!(chosen, format!("{prefix}x1"), "--name keeps the prefix");
            let core = seat_core(BridgeConfig::default(), &deck, &*mind);
            assert_eq!(core.disclosure(), mind.disclosure());
            for name in [named, chosen.as_str()] {
                assert!(core.disclosure().names(name), "the seat refuses «{name}»");
                assert!(!Disclosure::Llm.names(name), "«{name}» claims a model");
            }
        }
    }
}
