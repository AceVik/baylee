//! `baylee-seat`: a mind at a table, as an ordinary socket player.
//!
//! ```text
//! baylee-seat join <room> [--mind house|scripted|anthropic[:<model>]|openai:<model>]
//!     [--profile <name>] [--deck <file> | --acceptance <name>]
//! ```
//!
//! Signs in as a guest under the name its mind discloses (`HOUSE-house`,
//! `TEST-scripted`, `LLM-sonnet-5-5`), stores the deck,
//! takes a free chair in the room, says ready, waits for the host to start,
//! and plays the game to its end. A bridge a host's client starts
//! (`--tethered --chair <n> --chair-ticket`) instead reads the host's chair
//! ticket off its stdin and sits down on it, with no account of its own, so
//! it gets in where the gateway takes no guests. The gateway is `--gateway`, else
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
//!
//! A language model's model and limits may come from the settings file
//! instead (`--profile`, `--config`; `docs/llm-seat.md`), whose daily and
//! monthly caps the spend book holds across games: a game reserves what it
//! may spend before it sits down, and settles when it is over.

use anyhow::{Context as _, bail};
use baylee_ai::AIProfile;
use baylee_client_core::llmseat::DEFAULT_THINK_SECS;
use baylee_client_core::llmseat::keys::{self as llmseat_keys, KeyEntry, KeyStore};
use baylee_client_core::llmseat::ledger::Moment;
use baylee_client_core::llmseat::seating::Order;
use baylee_client_core::{say, say_err};
use baylee_protocol::v1::SeatMind;
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::config::{self, Overrides, Paths};
use baylee_seat::deck::Deck;
use baylee_seat::declare;
use baylee_seat::keys;
use baylee_seat::link::SeatLink;
use baylee_seat::llm::{self, AnswerMode, Price, Secret, Spec, Tally, scrub};
use baylee_seat::lobby::{Chair, ChairTicket, GuestSignIn, Lobby, Room, Session, seat_name};
use baylee_seat::seat::{BLITZ_SECS, Outcome};
use baylee_seat::show::Show;
use baylee_seat::spend::{self, Booked};
use baylee_seat::{BridgeConfig, Disclosure, HouseMind, Mind, ScriptedMind, SeatCore, Transcript};
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
    Join(Box<Join>),
    /// Keep, replace or forget a profile's key in the OS credential store,
    /// or say whether one is kept (never what).
    Key(KeyArgs),
}

#[derive(clap::Args)]
struct KeyArgs {
    /// What to do: `status` prints `set`, `absent` or `unavailable: …`;
    /// `set` reads the key from stdin, one line; `delete` forgets it.
    #[arg(value_enum)]
    action: KeyAction,
    /// The profile whose key it is, in the settings file.
    #[arg(long, required_unless_present = "key_env", conflicts_with_all = ["key_env", "host"])]
    profile: Option<String>,
    /// The settings file [default: `BAYLEE_SEAT_CONFIG`, else
    /// `llm-seat.json` in the client's config directory].
    #[arg(long)]
    config: Option<PathBuf>,
    /// The variable the key would otherwise be read from, with `--host`:
    /// the entry by its parts, as the client names it.
    #[arg(long, requires = "host")]
    key_env: Option<String>,
    /// The host (and port) the key goes to, with `--key-env`.
    #[arg(long, requires = "key_env")]
    host: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum KeyAction {
    Status,
    Set,
    Delete,
}

#[derive(Clone, clap::Args)]
#[allow(clippy::struct_excessive_bools)] // command-line switches, each its own
struct Join {
    /// The room's id, as the lobby lists it.
    room: String,
    /// The chair to take, numbered from 0 as the lobby lists it [default:
    /// the first free one].
    #[arg(long)]
    chair: Option<u32>,
    /// Held by the program that started it, through stdin: when stdin
    /// closes, the bridge stops as for ctrl-c, and leaves its chair if the
    /// game has not begun. A debug build also reads orders on it, one JSON
    /// line each, that change the mind from its next decision
    /// (`docs/llm-seat.md` §"A language model at your table").
    #[arg(long)]
    tethered: bool,
    /// Sit down on the host's chair ticket, the first line of stdin, rather
    /// than as a guest: the bridge a host's client starts, which gets in
    /// where the gateway takes no guests. The ticket is never an argument
    /// (`docs/llm-seat.md` §"A language model at your table").
    #[arg(long, requires_all = ["tethered", "chair"])]
    chair_ticket: bool,
    /// The gateway's address [default: `BAYLEE_GATEWAY`, else the local one].
    #[arg(long)]
    gateway: Option<String>,
    /// What decides: `house`, `scripted`, `anthropic[:<model>]` (key in
    /// `ANTHROPIC_API_KEY`), `openai:<model>` (key and address in
    /// `BAYLEE_LLM_API_KEY` and `BAYLEE_LLM_BASE_URL`), or an agent CLI
    /// signed in to a subscription, `cli:<tool>[:<model>]` (`cli:claude`,
    /// `cli:claude:opus`; no key) [default: the settings file's default
    /// profile, else house].
    #[arg(long, value_parser = MindKind::parse)]
    mind: Option<MindKind>,
    /// A profile of the settings file to play: its model, its limits and
    /// its key's variable, under any flag given here [default: the file's
    /// default profile].
    #[arg(long)]
    profile: Option<String>,
    /// The settings file (`docs/llm-seat.md`) [default:
    /// `BAYLEE_SEAT_CONFIG`, else `llm-seat.json` in the client's config
    /// directory]; a file named here must be there.
    #[arg(long)]
    config: Option<PathBuf>,
    /// The spend book that holds the settings file's daily and monthly caps
    /// [default: `llm-spend.json` beside the settings file, kept only when
    /// there is one].
    #[arg(long)]
    ledger: Option<PathBuf>,
    /// How hard a language model thinks (`low`, `medium`, `high`, …)
    /// [default: medium on Anthropic, the endpoint's own elsewhere].
    #[arg(long)]
    effort: Option<String>,
    /// Play the build's default effort (medium on Anthropic, the model's
    /// own elsewhere) whatever the profile names: for a model that does
    /// not take the profile's.
    #[arg(long, conflicts_with = "effort")]
    default_effort: bool,
    /// How an OpenAI-compatible model answers: by calling a tool, or with a
    /// JSON object, for a server without tools; `json-schema` asks for the
    /// object by its schema, for a server that refuses a bare `json`. A CLI
    /// answers `json-schema` or `json` [default: tools; json-schema for a
    /// CLI].
    #[arg(long, value_enum)]
    answer: Option<Answering>,
    /// The most tokens one reply may take, thinking included [default:
    /// 16000 on Anthropic, 8000 elsewhere].
    #[arg(long)]
    max_tokens: Option<u32>,
    /// The most a language model may spend on one game, in US dollars
    /// [default: 5]; past it the house finishes the game. It needs the
    /// model's price: this build's, or --price-in and --price-out. A model
    /// with no price is refused unless --spend-tokens states its limit.
    #[arg(long, value_parser = usd)]
    spend_usd: Option<f64>,
    /// The most tokens a language model may spend on one game, in and out
    /// together [default: 5000000; 20000000 for a CLI, whose cache reads
    /// count]; the only limit, and required, for a model this build has no
    /// price for and none is given.
    #[arg(long)]
    spend_tokens: Option<u64>,
    /// The most calls a language model may make in one game; past it the
    /// house finishes the game [default: 500 for a CLI, no limit for an
    /// API].
    #[arg(long)]
    spend_calls: Option<u64>,
    /// The model's input price in US dollars per million tokens, with
    /// --price-out: for a model this build has no price for, or over its
    /// own. Cache writes count at 1.25 times it and cache reads at all of
    /// it, so the dollar budget errs high.
    #[arg(long, requires = "price_out", value_parser = usd)]
    price_in: Option<f64>,
    /// The model's output price (thinking included) in US dollars per
    /// million tokens, with --price-in.
    #[arg(long, requires = "price_in", value_parser = usd)]
    price_out: Option<f64>,
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
    /// The longest one answer may take, whatever the table's clock allows
    /// [default: 60].
    #[arg(long)]
    think_secs: Option<u64>,
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
    /// With one JSON object, held to the answer's schema.
    JsonSchema,
}

/// A mind to play with, and what it spends, when it spends anything.
struct Chosen {
    mind: Arc<dyn Mind>,
    /// The name after the mind's prefix when `--name` gives none: the
    /// mind's kind, or a language model's tag.
    label: String,
    tally: Option<Arc<Mutex<Tally>>>,
    /// The game's reservation in the spend book, for a language model
    /// that plays under a settings file or `--ledger`.
    booked: Option<Booked>,
    /// The longest one answer may take, in seconds.
    think_secs: u64,
    /// What the player should be told about the choice.
    note: Option<String>,
    /// What the seat tells the table answers it, for the game's record.
    declared: SeatMind,
}

impl Join {
    /// What the command line says over a profile.
    fn overrides(&self) -> Overrides {
        Overrides {
            effort: self.effort.clone(),
            default_effort: self.default_effort,
            answer: self.answer.map(|answer| match answer {
                Answering::Tools => AnswerMode::Tools,
                Answering::Json => AnswerMode::Json,
                Answering::JsonSchema => AnswerMode::JsonSchema,
            }),
            max_tokens: self.max_tokens,
            price: self
                .price_in
                .zip(self.price_out)
                .map(|(input, output)| Price::per_million(input, output)),
            spend_usd: self.spend_usd,
            spend_tokens: self.spend_tokens,
            spend_calls: self.spend_calls,
            think_secs: self.think_secs,
        }
    }
}

/// The mind `join` plays with, at `level` where the house decides. A
/// language model's settings are the settings file's under the command
/// line ([`config::plan`]), found through `env`; its key comes from `env`
/// and nowhere else; and under a settings file (or `--ledger`) its game is
/// reserved in the spend book at `now` before anything else is done.
fn choose(
    join: &Join,
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
    level: AIProfile,
    now: Moment,
) -> anyhow::Result<Chosen> {
    let quiet = |mind: Arc<dyn Mind>, label: &str, declared: SeatMind| Chosen {
        mind,
        declared,
        label: label.into(),
        tally: None,
        booked: None,
        think_secs: join.think_secs.unwrap_or(DEFAULT_THINK_SECS),
        note: None,
    };
    let spec = match &join.mind {
        Some(kind @ (MindKind::House | MindKind::Scripted)) => {
            anyhow::ensure!(
                join.profile.is_none(),
                "--profile names a language model's settings, and --mind {} plays none",
                kind.label()
            );
            return Ok(match kind {
                MindKind::House => quiet(
                    Arc::new(HouseMind::new(level)),
                    "house",
                    declare::house(&join.level),
                ),
                _ => quiet(
                    Arc::new(ScriptedMind::idle()),
                    "scripted",
                    declare::scripted(),
                ),
            });
        }
        Some(MindKind::Llm(spec)) => Some(spec),
        None => None,
    };
    let paths = Paths::resolve(join.config.as_deref(), join.ledger.as_deref(), env);
    let file = paths.load().map_err(anyhow::Error::msg)?;
    let planned = config::plan(
        spec,
        file.as_ref(),
        &paths,
        join.profile.as_deref(),
        &join.overrides(),
    )
    .map_err(anyhow::Error::msg)?;
    let Some(plan) = planned else {
        // Nothing names a model: the house, as with no settings file.
        return Ok(quiet(
            Arc::new(HouseMind::new(level)),
            "house",
            declare::house(&join.level),
        ));
    };
    let mut plan = plan;
    // A key the environment does not hold may be kept in the OS credential
    // store for the address it goes to (`keys`); it is read as if it were
    // in its variable, and nowhere else.
    let stored = plan.key_env.as_deref().and_then(|key_env| {
        let provider = plan.settings.provider;
        let env_base = provider.base_env().and_then(env);
        let base = llmseat_keys::address(provider, plan.base_url.as_deref(), env_base.as_deref())?;
        keys::stored_key(keys, key_env, &base, env)
    });
    let env = |name: &str| {
        env(name).or_else(|| {
            (plan.key_env.as_deref() == Some(name))
                .then(|| stored.clone())
                .flatten()
        })
    };
    no_key_in(args, &env, plan.key_env.as_deref().as_slice())?;
    // What the mind is reached through is checked before the game reserves
    // anything: a missing key or program refuses the game, not the book.
    let access = llm::check(&plan, &env).map_err(anyhow::Error::msg)?;
    plan.settings.transcripts.clone_from(&join.transcripts);
    let caps = file.as_ref().map(|file| file.caps).unwrap_or_default();
    let mut booked = paths
        .book(file.is_some())
        .map(|book| {
            spend::reserve(
                &book,
                &caps,
                &mut plan.settings,
                plan.profile.as_deref(),
                now,
            )
        })
        .transpose()
        .map_err(anyhow::Error::msg)?;
    let declared = declare::llm(&plan.settings);
    let built = access.build(&plan);
    if let Some(booked) = &mut booked {
        booked.watch(Arc::clone(&built.tally));
    }
    Ok(Chosen {
        mind: built.mind,
        label: built.label,
        tally: Some(built.tally),
        booked,
        think_secs: plan.think_secs,
        note: plan.note,
        declared,
    })
}

impl MindKind {
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "house" => Ok(Self::House),
            "scripted" => Ok(Self::Scripted),
            other => match Spec::parse(other) {
                Some(spec) => spec.map(Self::Llm),
                None => Err(format!(
                    "«{}» is not a mind: house, scripted, anthropic[:<model>], openai:<model> or \
                     cli:<tool>[:<model>]",
                    scrub(other, None)
                )),
            },
        }
    }

    fn label(&self) -> String {
        match self {
            Self::House => "house".into(),
            Self::Scripted => "scripted".into(),
            Self::Llm(spec) => spec.tag(),
        }
    }
}

/// An amount of US dollars: a number, finite and not below zero.
fn usd(text: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|usd| usd.is_finite() && *usd >= 0.0)
        .ok_or_else(|| "an amount of US dollars, such as 2.5".to_string())
}

/// Refuses a command line that carries an API key: a key belongs in the
/// environment, where no process list and no shell history shows it. Says
/// which argument without printing it. The keys looked for are the
/// providers' variables' and those of the variables in `also` (a profile's
/// `key_env`).
fn no_key_in(
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
    also: &[&str],
) -> anyhow::Result<()> {
    let keys: Vec<Secret> = ["ANTHROPIC_API_KEY", "BAYLEE_LLM_API_KEY"]
        .iter()
        .chain(also)
        .filter_map(|name| env(name).as_deref().and_then(Secret::new))
        .collect();
    for (at, arg) in args.iter().enumerate() {
        let shaped = scrub(arg, None) != *arg;
        let named = keys.iter().any(|key| scrub(arg, Some(key)) != *arg);
        anyhow::ensure!(
            !(shaped || named),
            "argument {at} looks like an API key: set the key in the environment \
             (ANTHROPIC_API_KEY, BAYLEE_LLM_API_KEY, or the variable a profile's key_env names) \
             or keep it in this machine's credential store (`baylee-seat key set`), never on \
             the command line"
        );
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    let done = tokio::runtime::Runtime::new()
        .map_err(anyhow::Error::from)
        .and_then(|runtime| {
            let done = runtime.block_on(run());
            // A stopped game has settled by now (`stopped`). A request
            // still out, such as a lobby call waiting on its timeout, is not
            // waited for: a process that was asked to stop stops.
            runtime.shutdown_timeout(Duration::from_secs(1));
            done
        });
    match done {
        Ok(()) => std::process::ExitCode::SUCCESS,
        // As `main` returning the error would say it, without the panic
        // that saying it to a closed stderr would be (`quiet`): the last
        // line is what a host's client shows under the chair.
        Err(e) => {
            say_err!("Error: {e:?}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    // Armed before anything else: a stop signal meets the operating
    // system's default disposition (the process ends where it stands, its
    // reservation uncounted) until something registers a listener for it.
    // `Stoppers::arm` does that first, synchronously, so every one of them
    // stands before `choose` ever reserves a game's budget (`spend::reserve`,
    // printed as "reserved $…"); building `Chosen` no longer races the
    // listeners that catch a signal after it.
    let mut stoppers = Stoppers::arm();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    // One TLS provider, named: the crates that speak TLS here ask rustls for
    // a default rather than bringing one.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let env = |name: &str| std::env::var(name).ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    no_key_in(&args, &env, &[])?;
    let keys = keys::store(&env);
    match Cli::parse().command {
        Command::Key(key) => key_command(&key, &env, &*keys),
        Command::Join(join) => {
            let (mut let_go, orders, ticket) = if join.tethered {
                let held = tether::hold(join.chair_ticket);
                (Some(held.let_go), Some(held.orders), held.ticket)
            } else {
                (None, None, None)
            };
            let unstarted: Unstarted = Arc::default();
            // Stopped, the game is dropped where it stands, and its
            // reservation settles with what it spent (`spend::Booked`).
            let done = tokio::select! {
                biased;
                by = stoppers.stopped() => Err(anyhow::anyhow!(
                    "stopped by {by} before the game was over"
                )),
                () = tether::let_go(let_go.as_mut()) => Err(anyhow::anyhow!(
                    "the program that started this bridge let go of it before the game was over"
                )),
                done = join_and_play(&join, &args, &env, &keys, orders, ticket, &unstarted) => done,
            };
            if done.is_err() {
                leave_unstarted(&unstarted).await;
            }
            done
        }
    }
}

/// `baylee-seat key`: the entry of the profile named, then what was asked
/// of the store, printed as one line ([`key_line`]).
fn key_command(
    key: &KeyArgs,
    env: &dyn Fn(&str) -> Option<String>,
    store: &dyn KeyStore,
) -> anyhow::Result<()> {
    let line = key_line(key, env, store, &mut std::io::stdin().lock())?;
    say!("{line}");
    Ok(())
}

/// The entry `key` names: by its parts, or by a profile of the settings
/// file.
fn key_entry(key: &KeyArgs, env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<KeyEntry> {
    if let (Some(key_env), Some(host)) = (&key.key_env, &key.host) {
        return KeyEntry::named(key_env, host)
            .context("--key-env names a variable and --host a host, such as api.deepseek.com");
    }
    let name = key.profile.as_deref().unwrap_or_default();
    let paths = Paths::resolve(key.config.as_deref(), None, env);
    let file = paths.load().map_err(anyhow::Error::msg)?;
    let profile = file
        .as_ref()
        .and_then(|file| file.profile(name))
        .with_context(|| {
            format!(
                "the settings file {} has no profile «{name}»",
                paths.named_file()
            )
        })?;
    let env_base = profile.provider.base_env().and_then(env);
    llmseat_keys::entry(profile, env_base.as_deref())
        .with_context(|| format!("the profile «{name}» plays a CLI, which reads no key"))
}

/// What `key` does to `store`, the key (for `set`) read as one line from
/// `input`; the line to print. The key is never printed, and an error
/// never holds it.
fn key_line(
    key: &KeyArgs,
    env: &dyn Fn(&str) -> Option<String>,
    store: &dyn KeyStore,
    input: &mut dyn std::io::BufRead,
) -> anyhow::Result<String> {
    let entry = key_entry(key, env)?;
    match key.action {
        KeyAction::Status => {}
        KeyAction::Set => {
            store.available().map_err(anyhow::Error::msg)?;
            let mut line = String::new();
            // One line, and no more of it than a key could be.
            let most = llmseat_keys::KEY_BYTES as u64 + 2;
            let mut limited = std::io::Read::take(&mut *input, most);
            std::io::BufRead::read_line(&mut limited, &mut line)
                .context("read the key from stdin")?;
            let typed = line.trim_end_matches(['\n', '\r']);
            store.set(&entry, typed).map_err(anyhow::Error::msg)?;
        }
        KeyAction::Delete => {
            store.available().map_err(anyhow::Error::msg)?;
            store.delete(&entry).map_err(anyhow::Error::msg)?;
        }
    }
    Ok(llmseat_keys::state(store, &entry).line())
}

/// A chair taken in a room whose game has not begun: what gives it up.
struct Leaving {
    lobby: Lobby,
    by: Standing,
    room: String,
}

/// How the bridge came to its chair, which is what it asks the gateway
/// with afterwards.
#[derive(Clone)]
enum Standing {
    /// As a guest, under its own session.
    Guest(Session),
    /// On its host's chair ticket: it has the chair's seat token and
    /// nothing else.
    Delegated(Chair),
}

impl Standing {
    /// The session to take the chair back with, for a guest.
    fn session(&self) -> Option<Session> {
        match self {
            Self::Guest(session) => Some(session.clone()),
            Self::Delegated(_) => None,
        }
    }
}

/// The chair to give up if the bridge stops before its game begins.
type Unstarted = Arc<Mutex<Option<Leaving>>>;

/// Gives up the chair of a game that never began, so the room does not
/// keep a chair for a bridge that is gone: a host's client starts another
/// in it (`docs/llm-seat.md` §"A language model at your table"). Two
/// seconds at most; a gateway that does not answer keeps the chair.
async fn leave_unstarted(unstarted: &Unstarted) {
    let leaving = unstarted
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(Leaving { lobby, by, room }) = leaving {
        let left = async {
            match &by {
                Standing::Guest(session) => lobby.leave(session, &room).await,
                Standing::Delegated(chair) => lobby.leave_chair(chair).await,
            }
        };
        match tokio::time::timeout(Duration::from_secs(2), left).await {
            Ok(Ok(())) => say!("left the chair: the game had not begun"),
            Ok(Err(e)) => say_err!("the chair could not be given up: {e}"),
            Err(_) => say_err!("the chair could not be given up: the gateway did not answer"),
        }
    }
}

/// Stdin held by the program that started the bridge (`--tethered`).
mod tether {
    use baylee_client_core::llmseat::seating::{LIVE_CHANGES, ORDER_BYTES};
    use baylee_client_core::say;
    use tokio::io::AsyncBufReadExt as _;
    use tokio::sync::{mpsc, oneshot};

    /// What the program that started the bridge hands it on stdin.
    pub struct Held {
        /// Fires when stdin closes: the program let go.
        pub let_go: oneshot::Receiver<()>,
        /// Each later line, an order.
        pub orders: mpsc::UnboundedReceiver<String>,
        /// The first line, when the bridge was told a chair ticket comes
        /// there (`--chair-ticket`): handed over as it was read and to
        /// nothing else, never an order, never printed.
        pub ticket: Option<oneshot::Receiver<String>>,
    }

    /// Reads stdin until it closes: first the chair ticket when `ticket`
    /// says one comes, then each line an order (a debug build's only; a
    /// release build reads them and does nothing with them), and its end
    /// the program letting go.
    pub fn hold(ticket: bool) -> Held {
        let (gone, let_go) = oneshot::channel();
        let (send, orders) = mpsc::unbounded_channel();
        let (ticket_in, ticket_out) = if ticket {
            let (tx, rx) = oneshot::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
            if let Some(ticket_in) = ticket_in {
                // Taken whatever it is: an empty or closed stdin hands over
                // an empty line, which is refused as no ticket.
                let first = lines.next_line().await.ok().flatten().unwrap_or_default();
                let _ = ticket_in.send(first);
            }
            let mut said = false;
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                if !LIVE_CHANGES {
                    if !said {
                        say!("order refused: a release build changes no mind during a game");
                        said = true;
                    }
                    continue;
                }
                if line.len() > ORDER_BYTES {
                    say!("order refused: an order is one line of at most {ORDER_BYTES} bytes");
                    continue;
                }
                let _ = send.send(line);
            }
            let _ = gone.send(());
        });
        Held {
            let_go,
            orders,
            ticket: ticket_out,
        }
    }

    /// Waits for the program to let go; forever when nothing holds the
    /// bridge.
    pub async fn let_go(held: Option<&mut oneshot::Receiver<()>>) {
        match held {
            Some(held) => {
                let _ = held.await;
            }
            None => std::future::pending().await,
        }
    }
}

/// Every signal a process can be asked to stop by, registered with the
/// operating system the moment this is built (`arm`), well before anything
/// reserves a game's budget: ctrl-c; on unix also SIGTERM (`kill`,
/// `docker stop`, a service manager); on Windows also ctrl-break, its
/// console closing, and the user logging off or the machine shutting down,
/// where the system waits a few seconds for the process to finish. One
/// that cannot be registered (rare: an operating-system limit) is simply
/// never answered, as before. SIGHUP keeps its own meaning (a bridge under
/// `nohup` plays on), and SIGKILL can never be answered: a bridge stopped
/// by either counts its reservation in full.
struct Stoppers {
    #[cfg(unix)]
    interrupt: Option<tokio::signal::unix::Signal>,
    #[cfg(unix)]
    terminate: Option<tokio::signal::unix::Signal>,
    #[cfg(windows)]
    ctrl_c: Option<tokio::signal::windows::CtrlC>,
    #[cfg(windows)]
    ctrl_break: Option<tokio::signal::windows::CtrlBreak>,
    #[cfg(windows)]
    ctrl_close: Option<tokio::signal::windows::CtrlClose>,
    #[cfg(windows)]
    ctrl_logoff: Option<tokio::signal::windows::CtrlLogoff>,
    #[cfg(windows)]
    ctrl_shutdown: Option<tokio::signal::windows::CtrlShutdown>,
}

impl Stoppers {
    /// Registers every listener now, synchronously (each constructor here
    /// installs the operating system's own hook before returning); `None`
    /// only where that failed.
    fn arm() -> Self {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            Self {
                interrupt: signal(SignalKind::interrupt()).ok(),
                terminate: signal(SignalKind::terminate()).ok(),
            }
        }
        #[cfg(windows)]
        {
            use tokio::signal::windows::{
                ctrl_break, ctrl_c, ctrl_close, ctrl_logoff, ctrl_shutdown,
            };
            Self {
                ctrl_c: ctrl_c().ok(),
                ctrl_break: ctrl_break().ok(),
                ctrl_close: ctrl_close().ok(),
                ctrl_logoff: ctrl_logoff().ok(),
                ctrl_shutdown: ctrl_shutdown().ok(),
            }
        }
        #[cfg(not(any(unix, windows)))]
        Self {}
    }

    /// The first of these already-armed listeners to fire; forever on one
    /// that was never armed.
    async fn stopped(&mut self) -> &'static str {
        #[cfg(unix)]
        {
            /// Waits on an armed listener, or forever if it was never
            /// armed (`arm` failed to register it).
            async fn on(
                listener: &mut Option<tokio::signal::unix::Signal>,
                name: &'static str,
            ) -> &'static str {
                match listener {
                    Some(listener) => {
                        listener.recv().await;
                        name
                    }
                    None => std::future::pending().await,
                }
            }
            let Self {
                interrupt,
                terminate,
            } = self;
            tokio::select! {
                by = on(interrupt, "ctrl-c") => by,
                by = on(terminate, "SIGTERM") => by,
            }
        }
        #[cfg(windows)]
        {
            let Self {
                ctrl_c,
                ctrl_break,
                ctrl_close,
                ctrl_logoff,
                ctrl_shutdown,
            } = self;
            macro_rules! on {
                ($listener:expr, $name:expr) => {
                    async {
                        match $listener {
                            Some(listener) => {
                                listener.recv().await;
                                $name
                            }
                            None => std::future::pending().await,
                        }
                    }
                };
            }
            tokio::select! {
                by = on!(ctrl_c, "ctrl-c") => by,
                by = on!(ctrl_break, "ctrl-break") => by,
                by = on!(ctrl_close, "the console closing") => by,
                by = on!(ctrl_logoff, "the user logging off") => by,
                by = on!(ctrl_shutdown, "the machine shutting down") => by,
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            match tokio::signal::ctrl_c().await {
                Ok(_) => "ctrl-c",
                Err(_) => std::future::pending().await,
            }
        }
    }
}

/// Sits down, plays the game out, and says how it went.
async fn join_and_play(
    join: &Join,
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
    keys: &Arc<dyn KeyStore>,
    orders: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    ticket: Option<tokio::sync::oneshot::Receiver<String>>,
    unstarted: &Unstarted,
) -> anyhow::Result<()> {
    let mut seated = sit_down(join, args, env, &**keys, ticket, unstarted).await?;
    let tally = seated.tally.clone();
    let booked: Current = Arc::new(Mutex::new(seated.booked.take()));
    let show = (join.show || tally.is_some()).then(|| Arc::new(Mutex::new(Show::new())));
    let played = play_out(join, seated, show.clone(), keys, orders, &booked).await?;
    let booked = booked.lock().unwrap_or_else(PoisonError::into_inner).take();
    report(&played);
    if let Some(show) = show {
        let tally = tally.map(|t| t.lock().unwrap_or_else(PoisonError::into_inner).clone());
        let show = show.lock().unwrap_or_else(PoisonError::into_inner);
        for line in show.summary(&played.stats, tally.as_ref()) {
            say!("{line}");
        }
    }
    if let Some(mut booked) = booked {
        booked.settle(spend::now()).map_err(anyhow::Error::msg)?;
        say!("the game is settled in the spend book");
    }
    Ok(())
}

/// A chair taken, and what the seat plays with.
struct Seated {
    lobby: Lobby,
    by: Standing,
    chair: Chair,
    deck: Deck,
    profile: AIProfile,
    mind: Arc<dyn Mind>,
    tally: Option<Arc<Mutex<Tally>>>,
    booked: Option<Booked>,
    think_secs: u64,
    declared: SeatMind,
}

/// The name a chair played by `mind` signs in under: the prefix of what
/// the mind says it is, then `--name` or the mind's `label` (its kind, or
/// a language model's model: `LLM-sonnet-5-5`).
fn display_name(name: Option<&str>, label: &str, mind: &dyn Mind) -> anyhow::Result<String> {
    seat_name(mind.disclosure(), name.unwrap_or(label))
}

/// The seat that plays for `mind`, held at the table to the name
/// [`display_name`] gave the chair: both are what the mind says it is.
fn seat_core(config: BridgeConfig, deck: &Deck, mind: &dyn Mind) -> SeatCore {
    SeatCore::new(config, deck.list.clone(), mind.disclosure())
}

/// Whether `room` would seat this bridge: waiting, a chair free, its
/// password given where it is locked, and a clock a thinking mind can keep.
fn room_takes_us(room: &Room, join: &Join, has_password: bool) -> anyhow::Result<()> {
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
    if room.locked && !has_password {
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
    Ok(())
}

/// How long a bridge waits for the chair ticket on its stdin: the program
/// that started it writes the ticket as it starts it.
const TICKET_WAIT: Duration = Duration::from_secs(30);

/// How long the mind's check before ready may take ([`Mind::check`]): its
/// own calls give up after ten seconds; this bounds the whole.
const CHECK_WAIT: Duration = Duration::from_secs(30);

/// Chooses the mind (a language model's game reserved in the spend book
/// first), then takes the chair: on the host's chair ticket when one comes
/// on stdin (`--chair-ticket`), else as a guest who checks the room, stores
/// its deck, joins and says ready. Then waits for the host to start.
async fn sit_down(
    join: &Join,
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
    ticket: Option<tokio::sync::oneshot::Receiver<String>>,
    unstarted: &Unstarted,
) -> anyhow::Result<Seated> {
    let profile = AIProfile::named(&join.level)
        .with_context(|| format!("no house level called «{}»", join.level))?;
    let Chosen {
        mind,
        label,
        tally,
        booked,
        think_secs,
        note,
        declared,
    } = choose(join, args, env, keys, profile, spend::now())?;
    if let Some(note) = note {
        say!("{note}");
    }
    if let Some(booked) = &booked {
        say!("{}", booked.sat_down());
    }
    let display_name = display_name(join.name.as_deref(), &label, &*mind)?;
    let deck = match (&join.deck, &join.acceptance) {
        (Some(path), _) => Deck::from_file(path)?,
        (None, name) => Deck::acceptance(name.as_deref().unwrap_or("Allytifact"))?,
    };
    let gateway = join
        .gateway
        .clone()
        .or_else(|| std::env::var("BAYLEE_GATEWAY").ok())
        .unwrap_or_else(|| LOCAL_GATEWAY.to_string());
    let lobby = Lobby::new(&gateway);
    let (by, chair) = match ticket {
        Some(ticket) => {
            let line = tokio::time::timeout(TICKET_WAIT, ticket)
                .await
                .map_err(|_| anyhow::anyhow!("no chair ticket came on stdin"))?
                .map_err(|_| anyhow::anyhow!("stdin closed before a chair ticket came"))?;
            let ticket = ChairTicket::read(&line)?;
            let seat = join
                .chair
                .context("a chair ticket is for one chair: name it with --chair")?;
            let (chair, decide_secs) = lobby
                .redeem(&ticket, &join.room, seat, &display_name, &deck)
                .await?;
            let by = Standing::Delegated(chair.clone());
            // Until the game begins, a bridge that stops gives the chair up.
            *unstarted.lock().unwrap_or_else(PoisonError::into_inner) = Some(Leaving {
                lobby: lobby.clone(),
                by: by.clone(),
                room: chair.game_id.clone(),
            });
            if let Some(secs) = decide_secs.filter(|s| *s <= BLITZ_SECS)
                && !join.allow_blitz
            {
                bail!(
                    "the room {} gives {secs} s a question, too few for a mind that thinks \
                     (sit anyway with --allow-blitz)",
                    join.room
                );
            }
            (by, chair)
        }
        None => sit_as_guest(join, &lobby, &display_name, &deck, unstarted).await?,
    };
    say!(
        "«{display_name}» sits in chair {} of room {} with {}; waiting for the host to start",
        chair.seat,
        chair.game_id,
        deck.name
    );
    ready_and_wait(&*mind, &lobby, &by, &chair).await?;
    unstarted
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    say!("the game is on");
    Ok(Seated {
        lobby,
        by,
        chair,
        deck,
        profile,
        mind,
        tally,
        booked,
        think_secs,
        declared,
    })
}

/// Says the chair is ready once the mind answered its check, and waits for
/// the host to start.
///
/// Ready only then: a room never starts with a chair whose model cannot
/// play, and the chair's card says why not (the bridge's last line), the
/// chair given back as the bridge stops.
async fn ready_and_wait(
    mind: &dyn Mind,
    lobby: &Lobby,
    by: &Standing,
    chair: &Chair,
) -> anyhow::Result<()> {
    match tokio::time::timeout(CHECK_WAIT, mind.check()).await {
        Ok(Ok(())) => {}
        Ok(Err(why)) => bail!("the mind cannot play: {why}"),
        Err(_) => bail!(
            "the mind cannot play: it did not answer its check within {} s",
            CHECK_WAIT.as_secs()
        ),
    }
    match by {
        Standing::Guest(session) => lobby.ready(session, &chair.game_id).await?,
        Standing::Delegated(chair) => lobby.chair_ready(chair).await?,
    }
    say!("the mind answered its check: the chair is ready");
    match by {
        Standing::Guest(session) => {
            lobby
                .wait_for_start(session, &chair.game_id, Duration::from_secs(1))
                .await?;
        }
        Standing::Delegated(chair) => {
            lobby
                .wait_for_chair_start(chair, Duration::from_secs(1))
                .await?;
        }
    }
    Ok(())
}

/// Signs in as a guest under `display_name`, checks the room, stores the
/// deck and takes the chair (ready is said once the mind answered its
/// check, by [`ready_and_wait`]): the way in where no host handed
/// this bridge a chair ticket, on a gateway that takes guests.
async fn sit_as_guest(
    join: &Join,
    lobby: &Lobby,
    display_name: &str,
    deck: &Deck,
    unstarted: &Unstarted,
) -> anyhow::Result<(Standing, Chair)> {
    let password = join
        .password
        .clone()
        .or_else(|| std::env::var("BAYLEE_ROOM_PASSWORD").ok());
    let invite_key = join
        .invite_key
        .clone()
        .or_else(|| std::env::var("BAYLEE_INVITE_KEY").ok());
    let session = lobby
        .guest(&GuestSignIn {
            display_name: display_name.to_string(),
            invite_key,
        })
        .await?;
    let Some(room) = lobby.room(&session, &join.room).await? else {
        bail!("no room {} on {}", join.room, lobby.base());
    };
    room_takes_us(&room, join, password.is_some())?;
    let deck_id = lobby.upload(&session, deck).await?;
    let chair = lobby
        .join(
            &session,
            &room.id,
            &deck_id,
            password.as_deref(),
            join.chair,
        )
        .await?;
    let by = Standing::Guest(session.clone());
    // Until the game begins, a bridge that stops gives the chair up.
    *unstarted.lock().unwrap_or_else(PoisonError::into_inner) = Some(Leaving {
        lobby: lobby.clone(),
        by: by.clone(),
        room: room.id.clone(),
    });
    Ok((by, chair))
}

/// Plays the seated chair's game to its end, printing each decision when
/// there is a `show`.
async fn play_out(
    join: &Join,
    seated: Seated,
    show: Option<Arc<Mutex<Show>>>,
    keys: &Arc<dyn KeyStore>,
    orders: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    booked: &Current,
) -> anyhow::Result<bridge::Played> {
    // Orders change the mind from its next decision; the task that reads
    // them ends with the game.
    let (swaps, _reading) = orders
        .map(|orders| {
            let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
            let reader = OrderReader {
                join: join.clone(),
                disclosure: seated.mind.disclosure(),
                booked: Arc::clone(booked),
                keys: Arc::clone(keys),
            };
            (swaps, Aborting(tokio::spawn(reader.run(orders, send))))
        })
        .unzip();
    let config = BridgeConfig {
        think: Duration::from_secs(seated.think_secs),
        allow_blitz: join.allow_blitz,
        house: seated.profile,
        ..BridgeConfig::default()
    };
    let core = seat_core(config, &seated.deck, &*seated.mind).with_mind(seated.declared.clone());
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
                say!("{line}");
            }
        });
    }
    let options = PlayOptions {
        min_think: Duration::from_millis(join.min_think_ms),
        ..PlayOptions::default()
    };
    let mut link = SeatLink::new(seated.lobby, seated.chair, seated.by.session());
    let played = bridge::play_swapping(
        &mut link,
        core,
        seated.mind,
        &mut transcript,
        &options,
        swaps,
    )
    .await?;
    transcript.write_value(&serde_json::json!({ "summary": played.stats }));
    transcript.flush();
    Ok(played)
}

/// The game's reservation in the spend book, held by the mind that plays
/// now: a swapped-in mind's replaces it, which settles the old one.
type Current = Arc<Mutex<Option<Booked>>>;

/// A task that ends when this is dropped.
struct Aborting(tokio::task::JoinHandle<()>);

impl Drop for Aborting {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Reads a debug bridge's orders during its game and hands the bridge the
/// minds they name (`docs/llm-seat.md` §"Changing a chair during the
/// game"). Each mind is chosen as at sit-down ([`choose`]): its key or
/// program checked and its game reserved in the spend book before it is
/// handed over, and refused, with the old mind playing on, where either
/// fails. The chair keeps the name it sat down under, so a mind is taken
/// only where that name still tells the truth: one of the same kind, or the
/// house in a language model's chair (less than the name claims, never
/// more).
struct OrderReader {
    join: Join,
    disclosure: Disclosure,
    booked: Current,
    keys: Arc<dyn KeyStore>,
}

impl OrderReader {
    async fn run(
        self,
        mut orders: tokio::sync::mpsc::UnboundedReceiver<String>,
        send: tokio::sync::mpsc::UnboundedSender<bridge::Swap>,
    ) {
        let env = |name: &str| std::env::var(name).ok();
        while let Some(line) = orders.recv().await {
            match self.swap(&line, &env) {
                Ok((swap, label)) => {
                    if send.send(swap).is_err() {
                        return;
                    }
                    say!("order: {label} plays this chair from its next decision");
                }
                Err(why) => say!("order refused: {}", scrub(&format!("{why:#}"), None)),
            }
        }
    }

    /// The mind `line` orders, ready to hand over, and its label.
    fn swap(
        &self,
        line: &str,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> anyhow::Result<(bridge::Swap, String)> {
        let order = Order::parse(line).map_err(anyhow::Error::msg)?;
        // Asked before anything is reserved: what the order names is what
        // its mind will say it is.
        let kind = match order {
            Order::House { .. } => Disclosure::House,
            Order::Model { .. } => Disclosure::Llm,
        };
        anyhow::ensure!(
            kind == self.disclosure
                || (kind == Disclosure::House && self.disclosure == Disclosure::Llm),
            "this chair sat down as {}…, and a mind may play it only where that name tells the \
             truth",
            self.disclosure.prefix()
        );
        let join = self.ordered(&order, env)?;
        let level = AIProfile::named(&join.level)
            .with_context(|| format!("no house level called «{}»", join.level))?;
        let chosen = choose(
            &join,
            &[line.to_string()],
            env,
            &*self.keys,
            level,
            spend::now(),
        )?;
        anyhow::ensure!(
            chosen.mind.disclosure() == kind,
            "the order named a {kind:?} mind and another was chosen"
        );
        if let Some(booked) = &chosen.booked {
            say!("{}", booked.sat_down());
        }
        let current = Arc::clone(&self.booked);
        let next = chosen.booked;
        let taken: Box<dyn FnOnce() + Send> = Box::new(move || {
            let old = std::mem::replace(
                &mut *current.lock().unwrap_or_else(PoisonError::into_inner),
                next,
            );
            // The old mind has answered its last: its game settles.
            drop(old);
        });
        Ok((
            bridge::Swap {
                mind: chosen.mind,
                think: Some(Duration::from_secs(chosen.think_secs)),
                taken: Some(taken),
                declared: Some(chosen.declared),
            },
            chosen.label,
        ))
    }

    /// The command line the order amounts to: this bridge's, its mind,
    /// profile and effort replaced.
    fn ordered(&self, order: &Order, env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Join> {
        match order {
            Order::House { level } => Ok(Join {
                mind: Some(MindKind::House),
                profile: None,
                level: level.clone(),
                ..self.join.clone()
            }),
            Order::Model {
                profile, effort, ..
            } => {
                let paths = Paths::resolve(
                    self.join.config.as_deref(),
                    self.join.ledger.as_deref(),
                    env,
                );
                let file = paths.load().map_err(anyhow::Error::msg)?;
                let provider = file
                    .as_ref()
                    .and_then(|file| file.profile(profile))
                    .map(|found| found.provider)
                    .with_context(|| format!("the settings file has no profile «{profile}»"))?;
                let mind = order
                    .mind(provider)
                    .map(|mind| MindKind::parse(&mind))
                    .transpose()
                    .map_err(anyhow::Error::msg)?;
                Ok(Join {
                    mind,
                    profile: Some(profile.clone()),
                    effort: effort.clone(),
                    default_effort: effort.is_none(),
                    ..self.join.clone()
                })
            }
        }
    }
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
    say!(
        "{outcome} after {} turns: {} questions, {} answered by standing orders, {} wakes \
         and {} payment steps, {} questions in a plan; the mind answered {}, the house {}, the least answer {}; \
         {} fallbacks, {} refused, {} ms of model time, {} sockets",
        stats.turns,
        stats.questions,
        stats.standing.total(),
        stats.wakes,
        stats.continuations,
        stats.planned,
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
mod tests;
