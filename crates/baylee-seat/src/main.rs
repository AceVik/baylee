//! `baylee-seat`: a mind at a table, as an ordinary socket player.
//!
//! ```text
//! baylee-seat join <room> --mind house|scripted|anthropic[:<model>]|openai:<model>
//!     [--deck <file> | --acceptance <name>]
//! ```
//!
//! Signs in as a guest under the name its mind discloses (`HOUSE-house`,
//! `TEST-scripted`, `LLM-sonnet-5`), stores the deck,
//! takes a free chair in the room, says ready, waits for the host to start,
//! and plays the game to its end. The gateway is `--gateway`, else
//! `BAYLEE_GATEWAY`, else the local default; a closed beta's key is
//! `--invite-key` or `BAYLEE_INVITE_KEY`, a locked room's password
//! `--password` or `BAYLEE_ROOM_PASSWORD`. `RUST_LOG=info` says what the
//! bridge is doing; `--transcripts <dir>` keeps every question and answer as
//! JSON lines, and a language model's every message and reply beside them.
//!
//! A language model's key is read from the environment only
//! (`ANTHROPIC_API_KEY`; `BAYLEE_LLM_API_KEY` and `BAYLEE_LLM_BASE_URL` for
//! an OpenAI-compatible endpoint), and a command line that carries one is
//! refused. Its decisions are shown on the terminal as it plays
//! (`baylee_seat::show`), with what it spent at the end.

use anyhow::{Context as _, bail};
use baylee_ai::AIProfile;
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::llm::{
    AnswerMode, ApiMind, Provider, Secret, Settings, Spec, Tally, credentials, scrub,
};
use baylee_seat::lobby::{Chair, GuestSignIn, Lobby, Session, seat_name};
use baylee_seat::seat::{BLITZ_SECS, Outcome};
use baylee_seat::show::Show;
use baylee_seat::{BridgeConfig, HouseMind, Mind, ScriptedMind, SeatCore, Transcript};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
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
    /// What decides: `house`, `scripted`, `anthropic[:<model>]` (key in
    /// `ANTHROPIC_API_KEY`) or `openai:<model>` (key and address in
    /// `BAYLEE_LLM_API_KEY` and `BAYLEE_LLM_BASE_URL`).
    #[arg(long, default_value = "house", value_parser = MindKind::parse)]
    mind: MindKind,
    /// How hard a language model thinks (`low`, `medium`, `high`, …)
    /// [default: medium on Anthropic, the endpoint's own elsewhere].
    #[arg(long)]
    effort: Option<String>,
    /// How an OpenAI-compatible model answers: by calling a tool, or with a
    /// JSON object, for a server without tools.
    #[arg(long, value_enum, default_value_t = Answering::Tools)]
    answer: Answering,
    /// The most a language model may spend on one game, in US dollars; past
    /// it the house finishes the game.
    #[arg(long, default_value_t = 5.0)]
    spend_usd: f64,
    /// The most tokens a language model may spend on one game.
    #[arg(long, default_value_t = 5_000_000)]
    spend_tokens: u64,
    /// Print every decision on the terminal, for any mind (a language
    /// model's always are).
    #[arg(long)]
    show: bool,
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

/// What decides.
#[derive(Clone, Debug)]
enum MindKind {
    /// The house heuristic, on what the seat sees.
    House,
    /// The least answer to every question: a seat that plays nothing.
    Scripted,
    /// A language model behind an API.
    Llm(Spec),
}

/// How an OpenAI-compatible model answers.
#[derive(Clone, Copy, ValueEnum)]
enum Answering {
    /// By calling the `decide` tool.
    Tools,
    /// With one JSON object.
    Json,
}

/// A mind to play with, and what it spends, when it spends anything.
struct Chosen {
    mind: Arc<dyn Mind>,
    tally: Option<Arc<Mutex<Tally>>>,
}

impl MindKind {
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "house" => Ok(Self::House),
            "scripted" => Ok(Self::Scripted),
            other => match Spec::parse(other) {
                Some(spec) => spec.map(Self::Llm),
                None => Err(format!(
                    "«{}» is not a mind: house, scripted, anthropic[:<model>] or openai:<model>",
                    scrub(other, None)
                )),
            },
        }
    }

    /// The mind this choice plays with. A language model's key comes from
    /// `env` and nowhere else.
    fn mind(
        &self,
        profile: AIProfile,
        join: &Join,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> anyhow::Result<Chosen> {
        Ok(match self {
            Self::House => Chosen {
                mind: Arc::new(HouseMind::new(profile)),
                tally: None,
            },
            Self::Scripted => Chosen {
                mind: Arc::new(ScriptedMind::idle()),
                tally: None,
            },
            Self::Llm(spec) => {
                let mut settings = Settings::new(spec);
                if let Some(effort) = &join.effort {
                    anyhow::ensure!(
                        !effort.is_empty()
                            && effort.len() <= 10
                            && effort.chars().all(|c| c.is_ascii_lowercase()),
                        "an effort is a word such as low, medium or high"
                    );
                    settings.effort = Some(effort.clone());
                }
                if matches!(join.answer, Answering::Json) {
                    anyhow::ensure!(
                        spec.provider == Provider::OpenAi,
                        "--answer json is for an OpenAI-compatible endpoint; Anthropic's models \
                         answer with tools"
                    );
                    settings.answer = AnswerMode::Json;
                }
                settings.spend_usd = join.spend_usd;
                settings.spend_tokens = join.spend_tokens;
                settings.transcripts.clone_from(&join.transcripts);
                let credentials =
                    credentials(spec.provider, env).map_err(|why| anyhow::anyhow!(why))?;
                let mind = ApiMind::new(settings, credentials);
                let tally = mind.tally();
                Chosen {
                    mind: Arc::new(mind),
                    tally: Some(tally),
                }
            }
        })
    }

    fn label(&self) -> String {
        match self {
            Self::House => "house".into(),
            Self::Scripted => "scripted".into(),
            Self::Llm(spec) => spec.tag(),
        }
    }
}

/// Refuses a command line that carries an API key: a key belongs in the
/// environment, where no process list and no shell history shows it. Says
/// which argument without printing it.
fn no_key_in(args: &[String], env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<()> {
    let keys: Vec<Secret> = ["ANTHROPIC_API_KEY", "BAYLEE_LLM_API_KEY"]
        .iter()
        .filter_map(|name| env(name).as_deref().and_then(Secret::new))
        .collect();
    for (at, arg) in args.iter().enumerate() {
        let shaped = scrub(arg, None) != *arg;
        let named = keys.iter().any(|key| scrub(arg, Some(key)) != *arg);
        anyhow::ensure!(
            !(shaped || named),
            "argument {at} looks like an API key: set the key in the environment \
             (ANTHROPIC_API_KEY, BAYLEE_LLM_API_KEY), never on the command line"
        );
    }
    Ok(())
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
    let env = |name: &str| std::env::var(name).ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    no_key_in(&args, &env)?;
    match Cli::parse().command {
        Command::Join(join) => {
            let seated = sit_down(&join, &env).await?;
            let tally = seated.tally.clone();
            let show = (join.show || matches!(join.mind, MindKind::Llm(_)))
                .then(|| Arc::new(Mutex::new(Show::new())));
            let played = play_out(&join, seated, show.clone()).await?;
            report(&played);
            if let Some(show) = show {
                let tally = tally.map(|t| t.lock().unwrap_or_else(PoisonError::into_inner).clone());
                let show = show.lock().unwrap_or_else(PoisonError::into_inner);
                for line in show.summary(&played.stats, tally.as_ref()) {
                    println!("{line}");
                }
            }
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
    tally: Option<Arc<Mutex<Tally>>>,
}

/// The name a chair played by `mind` signs in under: the prefix of what
/// the mind says it is, then `--name` or the mind's kind (a language
/// model's is its model: `LLM-sonnet-5`).
fn display_name(name: Option<&str>, kind: &MindKind, mind: &dyn Mind) -> anyhow::Result<String> {
    seat_name(mind.disclosure(), name.unwrap_or(&kind.label()))
}

/// The seat that plays for `mind`, held at the table to the name
/// [`display_name`] gave the chair: both are what the mind says it is.
fn seat_core(config: BridgeConfig, deck: &Deck, mind: &dyn Mind) -> SeatCore {
    SeatCore::new(config, deck.list.clone(), mind.disclosure())
}

/// Signs in, checks the room, takes a chair, says ready and waits for the
/// host to start.
async fn sit_down(join: &Join, env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Seated> {
    let profile = AIProfile::named(&join.level)
        .with_context(|| format!("no house level called «{}»", join.level))?;
    let Chosen { mind, tally } = join.mind.mind(profile, join, env)?;
    let display_name = display_name(join.name.as_deref(), &join.mind, &*mind)?;
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
        tally,
    })
}

/// Plays the seated chair's game to its end, printing each decision when
/// there is a `show`.
async fn play_out(
    join: &Join,
    seated: Seated,
    show: Option<Arc<Mutex<Show>>>,
) -> anyhow::Result<bridge::Played> {
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
    if let Some(show) = show {
        transcript = transcript.echo(move |note| {
            let lines = show
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .note(note);
            for line in lines {
                println!("{line}");
            }
        });
    }
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
    fn join(mind: &str) -> Join {
        Cli::try_parse_from(["baylee-seat", "join", "TEST-room", "--mind", mind])
            .map(|cli| match cli.command {
                Command::Join(join) => join,
            })
            .expect("a command line")
    }

    /// A placeholder key, so a language-model mind can be built without
    /// one; nothing is ever sent with it.
    fn placeholder(name: &str) -> Option<String> {
        (name == "ANTHROPIC_API_KEY" || name == "BAYLEE_LLM_API_KEY")
            .then(|| "TEST-placeholder-key".to_string())
    }

    #[test]
    fn every_mind_sits_under_the_name_of_what_it_is() {
        let deck = Deck::acceptance("Victory").unwrap();
        let minds = [
            ("house", "HOUSE-house", "HOUSE-"),
            ("scripted", "TEST-scripted", "TEST-"),
            ("anthropic", "LLM-sonnet-5", "LLM-"),
            ("anthropic:claude-opus-5-5", "LLM-opus-5-5", "LLM-"),
            ("openai:deepseek-chat", "LLM-deepseek", "LLM-"),
        ];
        for (spec, named, prefix) in minds {
            let join = join(spec);
            let kind = &join.mind;
            let chosen_mind = kind
                .mind(AIProfile::default(), &join, &placeholder)
                .unwrap();
            let mind = &*chosen_mind.mind;
            assert_eq!(display_name(None, kind, mind).unwrap(), named);
            let chosen = display_name(Some("x1"), kind, mind).unwrap();
            assert_eq!(chosen, format!("{prefix}x1"), "--name keeps the prefix");
            let core = seat_core(BridgeConfig::default(), &deck, mind);
            assert_eq!(core.disclosure(), mind.disclosure());
            let llm = matches!(kind, MindKind::Llm(_));
            for name in [named, chosen.as_str()] {
                assert!(core.disclosure().names(name), "the seat refuses «{name}»");
                assert_eq!(Disclosure::Llm.names(name), llm, "«{name}» and a model");
            }
        }
    }

    /// A key on the command line is refused, by its shape or by being the
    /// environment's key, and the refusal does not repeat it.
    #[test]
    fn a_key_on_the_command_line_is_refused_without_being_printed() {
        let args = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
        let fine = args(&["join", "room", "--mind", "anthropic"]);
        assert!(no_key_in(&fine, &placeholder).is_ok());
        let shaped = args(&["join", "room", "--name", "sk-ant-api03-AAAABBBBCCCCDDDD"]);
        let refused = no_key_in(&shaped, &|_| None).unwrap_err().to_string();
        assert!(refused.contains("argument 3"), "{refused}");
        assert!(!refused.contains("AAAABBBB"), "{refused}");
        let same = args(&["join", "room", "--password", "TEST-placeholder-key"]);
        assert!(no_key_in(&same, &placeholder).is_err());
        // No key in the environment, no language model.
        let join = join("anthropic");
        let missing = join.mind.mind(AIProfile::default(), &join, &|_| None);
        assert!(missing.is_err());
    }
}
