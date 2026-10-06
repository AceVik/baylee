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
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::config::{self, Overrides, Paths};
use baylee_seat::deck::Deck;
use baylee_seat::keys;
use baylee_seat::link::SeatLink;
use baylee_seat::llm::{self, AnswerMode, Price, Secret, Spec, Tally, scrub};
use baylee_seat::lobby::{Chair, GuestSignIn, Lobby, Room, Session, seat_name};
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
    let quiet = |mind: Arc<dyn Mind>, label: &str| Chosen {
        mind,
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
                MindKind::House => quiet(Arc::new(HouseMind::new(level)), "house"),
                _ => quiet(Arc::new(ScriptedMind::idle()), "scripted"),
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
        return Ok(quiet(Arc::new(HouseMind::new(level)), "house"));
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
             (ANTHROPIC_API_KEY, BAYLEE_LLM_API_KEY, or the variable a profile's key_env names), \
             never on the command line"
        );
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let done = runtime.block_on(run());
    // A stopped game has settled by now (`stopped`). A request still out,
    // such as a lobby call waiting on its timeout, is not waited for: a
    // process that was asked to stop stops.
    runtime.shutdown_timeout(Duration::from_secs(1));
    done
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
            let (mut let_go, orders) = if join.tethered {
                let (let_go, orders) = tether::hold();
                (Some(let_go), Some(orders))
            } else {
                (None, None)
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
                done = join_and_play(&join, &args, &env, &keys, orders, &unstarted) => done,
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
    println!("{line}");
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
    session: Session,
    room: String,
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
    if let Some(Leaving {
        lobby,
        session,
        room,
    }) = leaving
    {
        match tokio::time::timeout(Duration::from_secs(2), lobby.leave(&session, &room)).await {
            Ok(Ok(())) => println!("left the chair: the game had not begun"),
            Ok(Err(e)) => eprintln!("the chair could not be given up: {e}"),
            Err(_) => eprintln!("the chair could not be given up: the gateway did not answer"),
        }
    }
}

/// Stdin held by the program that started the bridge (`--tethered`).
mod tether {
    use baylee_client_core::llmseat::seating::{LIVE_CHANGES, ORDER_BYTES};
    use tokio::io::AsyncBufReadExt as _;
    use tokio::sync::{mpsc, oneshot};

    /// Reads stdin until it closes: each line an order (a debug build's
    /// only; a release build reads them and does nothing with them), and
    /// its end the program letting go.
    pub fn hold() -> (oneshot::Receiver<()>, mpsc::UnboundedReceiver<String>) {
        let (gone, let_go) = oneshot::channel();
        let (send, orders) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
            let mut said = false;
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                if !LIVE_CHANGES {
                    if !said {
                        println!("order refused: a release build changes no mind during a game");
                        said = true;
                    }
                    continue;
                }
                if line.len() > ORDER_BYTES {
                    println!("order refused: an order is one line of at most {ORDER_BYTES} bytes");
                    continue;
                }
                let _ = send.send(line);
            }
            let _ = gone.send(());
        });
        (let_go, orders)
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
    unstarted: &Unstarted,
) -> anyhow::Result<()> {
    let mut seated = sit_down(join, args, env, &**keys, unstarted).await?;
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
            println!("{line}");
        }
    }
    if let Some(mut booked) = booked {
        booked.settle(spend::now()).map_err(anyhow::Error::msg)?;
        println!("the game is settled in the spend book");
    }
    Ok(())
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
    booked: Option<Booked>,
    think_secs: u64,
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

/// Chooses the mind (a language model's game reserved in the spend book
/// first), signs in, checks the room, takes a chair, says ready and waits
/// for the host to start.
async fn sit_down(
    join: &Join,
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
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
    } = choose(join, args, env, keys, profile, spend::now())?;
    if let Some(note) = note {
        println!("{note}");
    }
    if let Some(booked) = &booked {
        println!("{}", booked.sat_down());
    }
    let display_name = display_name(join.name.as_deref(), &label, &*mind)?;
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
    room_takes_us(&room, join, password.is_some())?;
    let deck_id = lobby.upload(&session, &deck).await?;
    let chair = lobby
        .join(
            &session,
            &room.id,
            &deck_id,
            password.as_deref(),
            join.chair,
        )
        .await?;
    // Until the game begins, a bridge that stops gives the chair up.
    *unstarted.lock().unwrap_or_else(PoisonError::into_inner) = Some(Leaving {
        lobby: lobby.clone(),
        session: session.clone(),
        room: room.id.clone(),
    });
    lobby.ready(&session, &room.id).await?;
    println!(
        "«{display_name}» sits in chair {} of room {} with {}; waiting for the host to start",
        chair.seat, room.id, deck.name
    );
    lobby
        .wait_for_start(&session, &room.id, Duration::from_secs(1))
        .await?;
    unstarted
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    println!("the game is on");
    Ok(Seated {
        lobby,
        session,
        chair,
        deck,
        profile,
        mind,
        tally,
        booked,
        think_secs,
    })
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
                    println!("order: {label} plays this chair from its next decision");
                }
                Err(why) => println!("order refused: {}", scrub(&format!("{why:#}"), None)),
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
            println!("{}", booked.sat_down());
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
                taken: Some(taken),
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
    println!(
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
mod tests {
    use super::*;
    use baylee_client_core::llmseat::keys::MemoryKeys;
    use baylee_client_core::llmseat::ledger::{Book, Budget};
    use baylee_seat::Disclosure;
    use std::path::Path;

    /// 2026-09-30 12:00 UTC, two hours east.
    const NOW: Moment = Moment {
        unix: 1_790_769_600,
        offset: Some(7200),
    };

    /// A chair's name tells the truth about its mind (the owner's rule):
    /// the house signs in as the house and a script as a test, never as a
    /// language model, and the seat holds the table to that same name.
    fn join(mind: &str) -> Join {
        join_with(mind, &[]).expect("a command line")
    }

    /// `join` with more arguments after `--mind`.
    fn join_with(mind: &str, more: &[&str]) -> Result<Join, clap::Error> {
        let head = ["--mind", mind];
        join_args(&head.iter().chain(more).copied().collect::<Vec<_>>())
    }

    /// `join TEST-room` with `args`.
    fn join_args(args: &[&str]) -> Result<Join, clap::Error> {
        let head = ["baylee-seat", "join", "TEST-room"];
        Cli::try_parse_from(head.iter().chain(args)).map(|cli| match cli.command {
            Command::Join(join) => *join,
            Command::Key(_) => panic!("not a join"),
        })
    }

    /// A placeholder key, so a language-model mind can be built without
    /// one; nothing is ever sent with it. No other variable is set: no
    /// config directory, so no settings file, as on a machine without one.
    fn placeholder(name: &str) -> Option<String> {
        (name == "ANTHROPIC_API_KEY" || name == "BAYLEE_LLM_API_KEY")
            .then(|| "TEST-placeholder-key".to_string())
    }

    fn chosen(join: &Join, env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Chosen> {
        choose(
            join,
            &[],
            env,
            &MemoryKeys::default(),
            AIProfile::default(),
            NOW,
        )
    }

    /// A directory of this test's own, empty.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("baylee-seat-main-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A settings file at `dir/llm-seat.json`: a default Sonnet profile of
    /// $2 a game under a day's cap of $3.
    fn settings_in(dir: &Path) -> PathBuf {
        let path = dir.join("llm-seat.json");
        std::fs::write(
            &path,
            r#"{
              "default": "sonnet",
              "caps": {"day_usd": 3},
              "profiles": {
                "sonnet": {"provider": "anthropic", "model": "claude-sonnet-5-5",
                           "game_usd": 2, "think_secs": 30},
                "keyed": {"provider": "anthropic", "model": "claude-opus-5-5",
                          "key_env": "TEST_OWN_KEY"}
              }
            }"#,
        )
        .unwrap();
        path
    }

    /// The file's content and each file's bytes in `dir`, to see that
    /// nothing there was touched.
    fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                (name, std::fs::read(&path).unwrap())
            })
            .collect();
        files.sort();
        files
    }

    #[test]
    fn every_mind_sits_under_the_name_of_what_it_is() {
        let deck = Deck::acceptance("Victory").unwrap();
        let minds = [
            ("house", "HOUSE-house", "HOUSE-"),
            ("scripted", "TEST-scripted", "TEST-"),
            ("anthropic", "LLM-sonnet-5-5", "LLM-"),
            ("anthropic:claude-opus-5-5", "LLM-opus-5-5", "LLM-"),
            ("openai:deepseek-chat", "LLM-deepseek", "LLM-"),
        ];
        for (spec, named, prefix) in minds {
            // A model with no price sits down only with a token budget.
            let join = join_with(spec, &["--spend-tokens", "100000"]).unwrap();
            let chosen = chosen(&join, &placeholder).unwrap();
            let mind = &*chosen.mind;
            assert_eq!(display_name(None, &chosen.label, mind).unwrap(), named);
            let named_too = display_name(Some("x1"), &chosen.label, mind).unwrap();
            assert_eq!(named_too, format!("{prefix}x1"), "--name keeps the prefix");
            let core = seat_core(BridgeConfig::default(), &deck, mind);
            assert_eq!(core.disclosure(), mind.disclosure());
            let llm = matches!(join.mind, Some(MindKind::Llm(_)));
            for name in [named, named_too.as_str()] {
                assert!(core.disclosure().names(name), "the seat refuses «{name}»");
                assert_eq!(Disclosure::Llm.names(name), llm, "«{name}» and a model");
            }
        }
    }

    /// A key on the command line is refused, by its shape or by being the
    /// environment's key (a profile's own variable's too), and the refusal
    /// does not repeat it.
    #[test]
    fn a_key_on_the_command_line_is_refused_without_being_printed() {
        let args = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
        let fine = args(&["join", "room", "--mind", "anthropic"]);
        assert!(no_key_in(&fine, &placeholder, &[]).is_ok());
        let shaped = args(&["join", "room", "--name", "sk-ant-api03-AAAABBBBCCCCDDDD"]);
        let refused = no_key_in(&shaped, &|_| None, &[]).unwrap_err().to_string();
        assert!(refused.contains("argument 3"), "{refused}");
        assert!(!refused.contains("AAAABBBB"), "{refused}");
        let same = args(&["join", "room", "--password", "TEST-placeholder-key"]);
        assert!(no_key_in(&same, &placeholder, &[]).is_err());
        let own = |name: &str| (name == "TEST_OWN_KEY").then(|| "TEST-own-key-value".to_string());
        let theirs = args(&["join", "room", "--password", "TEST-own-key-value"]);
        assert!(
            no_key_in(&theirs, &own, &[]).is_ok(),
            "not a variable it knows"
        );
        assert!(no_key_in(&theirs, &own, &["TEST_OWN_KEY"]).is_err());
        // No key in the environment, no language model.
        let missing = chosen(&join("anthropic"), &|_| None);
        assert!(missing.is_err());
    }

    /// A model this build has no price for does not sit down under a
    /// dollar budget nobody can hold: it states its price, or a token
    /// budget as its limit, and `--help` says so.
    #[test]
    fn a_model_with_no_price_states_its_price_or_its_token_limit() {
        let sits = |mind: &str, more: &[&str]| {
            let join = join_with(mind, more).expect("a command line");
            chosen(&join, &placeholder)
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        let unpriced = "openai:deepseek-chat";
        let refused = sits(unpriced, &[]).unwrap_err();
        for named in [
            "deepseek-chat",
            "--price-in",
            "--price-out",
            "--spend-tokens",
        ] {
            assert!(refused.contains(named), "{refused}");
        }
        assert_eq!(refused.lines().count(), 1, "one sentence: {refused}");
        // A dollar budget for it is refused, token budget or not.
        for more in [
            &["--spend-usd", "3"][..],
            &["--spend-usd", "3", "--spend-tokens", "9"],
        ] {
            let refused = sits(unpriced, more).unwrap_err();
            assert!(refused.contains("--price-in"), "{refused}");
        }
        // The accepted forms: a token limit, a price (with or without a
        // budget of its own), and a model this build has a price for.
        sits(unpriced, &["--spend-tokens", "200000"]).unwrap();
        sits(unpriced, &["--price-in", "0.3", "--price-out", "1.2"]).unwrap();
        let priced = [
            "--price-in",
            "0.3",
            "--price-out",
            "1.2",
            "--spend-usd",
            "2",
        ];
        sits(unpriced, &priced).unwrap();
        sits("anthropic", &[]).unwrap();
        sits("anthropic:claude-opus-5-5", &["--spend-usd", "1"]).unwrap();
        // A price is both halves, and an amount of dollars.
        assert!(join_with(unpriced, &["--price-in", "0.3"]).is_err());
        assert!(join_with(unpriced, &["--price-out", "1.2"]).is_err());
        for bad in ["-1", "NaN", "inf", "a"] {
            let price_in = format!("--price-in={bad}");
            let more = [price_in.as_str(), "--price-out", "1"];
            assert!(join_with(unpriced, &more).is_err(), "{bad}");
            let spend = format!("--spend-usd={bad}");
            assert!(join_with("anthropic", &[&spend]).is_err(), "{bad}");
        }
        // What `join --help` says about it.
        let mut cli = <Cli as clap::CommandFactory>::command();
        let help = cli
            .find_subcommand_mut("join")
            .expect("join")
            .render_long_help()
            .to_string();
        let help = help.split_whitespace().collect::<Vec<_>>().join(" ");
        for said in [
            "A model with no price is refused unless --spend-tokens states its limit",
            "required, for a model this build has no price for",
            "--price-in",
            "--price-out",
            "--profile",
            "BAYLEE_SEAT_CONFIG",
            "llm-spend.json",
        ] {
            assert!(help.contains(said), "--help lacks «{said}»:\n{help}");
        }
    }

    /// With no settings file the bridge is what it was: the house when told
    /// nothing, a model with the build's limits and no spend book, and a
    /// profile it cannot have.
    #[test]
    fn with_no_settings_file_the_bridge_plays_as_before() {
        let bare = chosen(&join_args(&[]).unwrap(), &placeholder).unwrap();
        assert_eq!(bare.label, "house");
        assert!(bare.tally.is_none() && bare.booked.is_none());
        assert_eq!(bare.think_secs, DEFAULT_THINK_SECS);
        let model = chosen(&join("anthropic"), &placeholder).unwrap();
        assert!(model.tally.is_some());
        assert!(model.booked.is_none(), "no file, no book");
        assert_eq!(model.think_secs, 60);
        let refused = chosen(&join_args(&["--profile", "sonnet"]).unwrap(), &placeholder)
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert!(
            refused.contains("--profile sonnet") && refused.contains("none"),
            "{refused}"
        );
        let refused = chosen(
            &join_with("house", &["--profile", "x"]).unwrap(),
            &placeholder,
        )
        .map(|_| ())
        .unwrap_err()
        .to_string();
        assert!(refused.contains("--mind house plays none"), "{refused}");
    }

    /// Given `--config` (or `BAYLEE_SEAT_CONFIG`), the player's own config
    /// directory is neither read nor written, whatever it holds: here a
    /// file that would refuse if it were read, beside which no book grows.
    #[test]
    fn a_named_file_keeps_the_real_config_directory_out_of_it() {
        let dir = scratch("real-dir");
        let real = dir.join("xdg").join("baylee");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("llm-seat.json"), r#"{"caps": {"week_usd": 1}}"#).unwrap();
        let before = snapshot(&real);
        let mine = dir.join("mine");
        std::fs::create_dir_all(&mine).unwrap();
        let config = settings_in(&mine);
        let xdg = dir.join("xdg").display().to_string();
        let env = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some(xdg.clone()),
            _ => placeholder(name),
        };
        // The premise: without the flag, the real directory is where the
        // bridge looks, and its file is refused.
        let refused = chosen(&join_args(&[]).unwrap(), &env)
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert!(refused.contains("week_usd"), "{refused}");

        let named = join_args(&["--config", config.to_str().unwrap()]).unwrap();
        let game = chosen(&named, &env).expect("the named file plays");
        assert_eq!(game.label, "sonnet-5-5");
        assert_eq!(game.think_secs, 30);
        drop(game);
        let by_env = |name: &str| match name {
            "BAYLEE_SEAT_CONFIG" => Some(config.display().to_string()),
            _ => env(name),
        };
        drop(chosen(&join_args(&[]).unwrap(), &by_env).expect("the file the environment names"));
        assert_eq!(snapshot(&real), before, "the real directory is untouched");
        let book = Book::beside(&config).read().unwrap();
        assert_eq!(
            book.games.len(),
            2,
            "both games are in the book beside the named file"
        );
        assert!(book.games.iter().all(|g| g.settled.is_some()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file named must be there: a mistyped path never drops the caps.
    #[test]
    fn a_named_settings_file_must_be_there() {
        let dir = scratch("missing");
        let gone = dir.join("nothing-here.json");
        let named = join_args(&["--config", gone.to_str().unwrap()]).unwrap();
        let refused = chosen(&named, &placeholder)
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert!(refused.contains("there is no settings file"), "{refused}");
        let by_env = |name: &str| match name {
            "BAYLEE_SEAT_CONFIG" => Some(gone.display().to_string()),
            _ => placeholder(name),
        };
        assert!(chosen(&join("anthropic"), &by_env).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Under the file, a game reserves before it sits down and plays under
    /// what it was granted; with the day's cap taken it is refused with a
    /// sentence, and a game that ended gives back what it did not spend.
    #[test]
    fn under_a_settings_file_a_game_reserves_before_it_sits_down() {
        let dir = scratch("reserve");
        let config = settings_in(&dir);
        let named = join_args(&["--config", config.to_str().unwrap()]).unwrap();
        let first = chosen(&named, &placeholder).unwrap();
        assert_eq!(
            first.booked.as_ref().unwrap().grant().budget,
            Budget::Usd(2.0)
        );
        let second = chosen(&named, &placeholder).unwrap();
        let left = match second.booked.as_ref().unwrap().grant().budget {
            Budget::Usd(usd) => usd,
            Budget::Tokens(_) => panic!("dollars"),
        };
        assert!((left - 1.0).abs() < 1e-9, "what the day's cap left: {left}");
        let refused = chosen(&named, &placeholder)
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert!(refused.contains("the day's cap of $3.00"), "{refused}");
        assert!(refused.contains("local time, UTC+02:00"), "{refused}");
        drop(first);
        let third = chosen(&named, &placeholder).expect("the first game gave its $2 back");
        drop((second, third));

        // --mind over the file still counts in its book; the house does not.
        let model = join_with(
            "anthropic:claude-opus-5-5",
            &["--config", config.to_str().unwrap()],
        )
        .unwrap();
        assert!(chosen(&model, &placeholder).unwrap().booked.is_some());
        let house = join_with("house", &["--config", config.to_str().unwrap()]).unwrap();
        assert!(chosen(&house, &placeholder).unwrap().booked.is_none());
        let games = Book::beside(&config).read().unwrap().games.len();
        assert_eq!(games, 4, "the refusal reserved nothing, the house nothing");

        // A profile's key comes from its own variable.
        let keyed =
            join_args(&["--config", config.to_str().unwrap(), "--profile", "keyed"]).unwrap();
        let refused = chosen(&keyed, &placeholder)
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert!(refused.contains("set TEST_OWN_KEY"), "{refused}");
        let own = |name: &str| (name == "TEST_OWN_KEY").then(|| "TEST-own-key-value".to_string());
        assert!(chosen(&keyed, &own).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An agent CLI sits under its tool's and model's name and reads no
    /// key; its program is checked before the game reserves anything, so a
    /// CLI that is not there costs the book nothing. The program here is
    /// the test itself: found, and never started.
    #[test]
    fn a_cli_sits_with_no_key_and_is_checked_before_it_reserves() {
        let dir = scratch("cli");
        let config = dir.join("llm-seat.json");
        let program = std::env::current_exe().unwrap();
        let file = serde_json::json!({
            "default": "cc",
            "caps": {"day_tokens": 50_000_000},
            "profiles": {
                "cc": {"provider": "cli", "model": "claude:opus", "game_calls": 200,
                       "command": program},
                "gone": {"provider": "cli", "model": "claude",
                         "command": dir.join("nowhere").join("claude")}
            }
        });
        std::fs::write(&config, file.to_string()).unwrap();
        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let env = |name: &str| (name == "PATH").then(|| empty.display().to_string());
        let path = config.to_str().unwrap();
        let cc = join_args(&["--config", path]).unwrap();
        let chosen_cc = chosen(&cc, &env).unwrap();
        assert_eq!(chosen_cc.label, "claude-opus");
        let mind = &*chosen_cc.mind;
        assert_eq!(
            display_name(None, &chosen_cc.label, mind).unwrap(),
            "LLM-claude-opus"
        );
        assert_eq!(mind.disclosure(), Disclosure::Llm);
        assert_eq!(
            chosen_cc.booked.as_ref().unwrap().grant().budget,
            Budget::Tokens(20_000_000)
        );
        let tally = chosen_cc.tally.as_ref().unwrap();
        assert_eq!(tally.lock().unwrap().calls_cap, Some(200));
        drop(chosen_cc);
        for (join, said) in [
            (
                join_args(&["--config", path, "--profile", "gone"]).unwrap(),
                "is not a program this user may run",
            ),
            // With no profile to name it, the tool is looked for on PATH.
            (join("cli:claude"), "claude is not on PATH"),
        ] {
            let refused = chosen(&join, &env).map(|_| ()).unwrap_err().to_string();
            assert!(refused.contains(said), "{refused}");
        }
        let games = Book::beside(&config).read().unwrap().games.len();
        assert_eq!(games, 1, "a CLI that is not there reserved nothing");
        // The flag over the profile's cap.
        let flagged = join_args(&["--config", path, "--spend-calls", "7"]).unwrap();
        let tally = chosen(&flagged, &env).unwrap().tally.unwrap();
        assert_eq!(tally.lock().unwrap().calls_cap, Some(7));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `--ledger` puts the book where it says, file or no file.
    #[test]
    fn the_book_is_where_ledger_says() {
        let dir = scratch("ledger");
        let book = dir.join("elsewhere.json");
        let named = join_with("anthropic", &["--ledger", book.to_str().unwrap()]).unwrap();
        drop(chosen(&named, &placeholder).unwrap());
        assert_eq!(Book::new(book).read().unwrap().games.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A reader of orders for a bridge started as `join` would be, seated
    /// as `disclosure`.
    fn reader(join: Join, disclosure: Disclosure, booked: Option<Booked>) -> OrderReader {
        OrderReader {
            join,
            disclosure,
            booked: Arc::new(Mutex::new(booked)),
            keys: Arc::new(MemoryKeys::default()),
        }
    }

    /// An order is chosen as a sit-down is: the profile's provider puts the
    /// model in `--mind`, the effort over the profile's (none named: the
    /// build's default), its game reserved in the book; and the old
    /// reservation settles only once the new mind is taken.
    #[test]
    fn an_order_is_chosen_as_a_sit_down_and_settles_the_old_game_when_taken() {
        let dir = scratch("orders");
        let config = settings_in(&dir);
        let path = config.to_str().unwrap();
        let started = join_args(&["--config", path, "--profile", "sonnet", "--tethered"]).unwrap();
        let first = chosen(&started, &placeholder).unwrap();
        assert_eq!(first.mind.disclosure(), Disclosure::Llm);
        let reading = reader(started, Disclosure::Llm, first.booked);
        let book = Book::beside(&config);
        assert_eq!(book.read().unwrap().games.len(), 1);
        // An order for Opus over the same profile, at the build's effort.
        let line = Order::Model {
            profile: "sonnet".into(),
            model: "claude-opus-5-5".into(),
            effort: None,
        }
        .line();
        let order = Order::parse(&line).unwrap();
        let join = reading.ordered(&order, &placeholder).unwrap();
        assert!(matches!(&join.mind, Some(MindKind::Llm(spec)) if spec.model == "claude-opus-5-5"));
        assert!(join.default_effort && join.effort.is_none());
        assert_eq!(join.profile.as_deref(), Some("sonnet"));
        let (swap, label) = reading.swap(&line, &placeholder).unwrap();
        assert_eq!(label, "opus-5-5");
        let games = book.read().unwrap().games;
        assert_eq!(games.len(), 2, "reserved before it is handed over");
        assert!(games.iter().all(|g| g.settled.is_none()));
        (swap.taken.unwrap())();
        let games = book.read().unwrap().games;
        assert!(
            games[0].settled.is_some(),
            "the old game settles when taken"
        );
        assert!(games[1].settled.is_none(), "the new one plays on");
        // The house may play a language model's chair.
        let (_, label) = reading
            .swap(r#"{"mind":"house","level":"sharp"}"#, &placeholder)
            .unwrap();
        assert_eq!(label, "house");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An order the chair's name would lie about, a profile the file does
    /// not have, a key and a missing one are refused, each in a sentence,
    /// and nothing is reserved.
    #[test]
    fn an_order_the_chair_cannot_take_is_refused() {
        let dir = scratch("orders-refused");
        let config = settings_in(&dir);
        let path = config.to_str().unwrap();
        let started = join_args(&["--config", path, "--tethered"]).unwrap();
        let model = Order::Model {
            profile: "sonnet".into(),
            model: "claude-sonnet-5-5".into(),
            effort: Some("high".into()),
        }
        .line();
        let house = reader(started.clone(), Disclosure::House, None);
        let why = house.swap(&model, &placeholder).unwrap_err().to_string();
        assert!(why.contains("HOUSE-"), "{why}");
        let llm = reader(started, Disclosure::Llm, None);
        let gone = r#"{"mind":"model","profile":"gone","model":"claude-opus-5-5"}"#;
        let why = llm.swap(gone, &placeholder).unwrap_err().to_string();
        assert!(why.contains("no profile «gone»"), "{why}");
        let why = llm.swap(&model, &|_| None).unwrap_err().to_string();
        assert!(why.contains("ANTHROPIC_API_KEY"), "{why}");
        let key = format!(r#"{{"mind":"house","level":"sk-ant-{}"}}"#, "B".repeat(30));
        let why = llm.swap(&key, &placeholder).unwrap_err().to_string();
        assert!(!why.contains("BBBB"), "{why}");
        assert_eq!(Book::beside(&config).read().unwrap().games.len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `baylee-seat key` as the client runs it, against a store in memory.
    fn key_args(args: &[&str]) -> KeyArgs {
        let head = ["baylee-seat", "key"];
        match Cli::try_parse_from(head.iter().chain(args))
            .unwrap()
            .command
        {
            Command::Key(key) => key,
            Command::Join(_) => panic!("not a key command"),
        }
    }

    /// The client keeps, replaces and forgets a key through the bridge,
    /// which reads it from stdin and answers only whether one is kept; a
    /// CLI's profile has none, and a store that cannot be opened says so.
    #[test]
    fn a_key_is_kept_and_forgotten_and_never_printed() {
        let dir = scratch("key-command");
        let file = settings_in(&dir);
        let config = file.to_string_lossy().into_owned();
        let store = MemoryKeys::default();
        let none = |_: &str| None;
        let run = |action: &str, profile: &str, typed: &str, store: &MemoryKeys| {
            let args = key_args(&[action, "--profile", profile, "--config", &config]);
            key_line(&args, &none, store, &mut typed.as_bytes())
        };
        assert_eq!(run("status", "keyed", "", &store).unwrap(), "absent");
        let said = run("set", "keyed", "TEST-kept-0123456789abcdef\n", &store).unwrap();
        assert_eq!(said, "set");
        let at = KeyEntry::new("TEST_OWN_KEY", "https://api.anthropic.com").unwrap();
        assert_eq!(
            store.get(&at).unwrap().as_deref(),
            Some("TEST-kept-0123456789abcdef"),
            "under its variable and its host"
        );
        let refused = run("set", "keyed", "two words\n", &store).unwrap_err();
        assert!(!format!("{refused:#}").contains("two"), "{refused:#}");
        assert_eq!(run("delete", "keyed", "", &store).unwrap(), "absent");
        assert!(run("status", "nobody", "", &store).is_err());
        let off = MemoryKeys::unavailable("TEST: no store here");
        assert_eq!(
            run("status", "keyed", "", &off).unwrap(),
            "unavailable: TEST: no store here"
        );
        assert!(run("set", "keyed", "TEST-kept-0123456789abcdef\n", &off).is_err());
        // By its parts, as the client names it: the same entry.
        let parts = key_args(&[
            "status",
            "--key-env",
            "TEST_OWN_KEY",
            "--host",
            "api.anthropic.com",
        ]);
        store.set(&at, "TEST-kept-0123456789abcdef").unwrap();
        assert_eq!(
            key_line(&parts, &none, &store, &mut "".as_bytes()).unwrap(),
            "set"
        );
        let bad = key_args(&["status", "--key-env", "TEST_OWN_KEY", "--host", "a/b"]);
        assert!(key_line(&bad, &none, &store, &mut "".as_bytes()).is_err());
        assert!(
            Cli::try_parse_from(["baylee-seat", "key", "status", "--key-env", "X"]).is_err(),
            "a variable without its host"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A key kept in the store sits a language model down where its
    /// variable is unset, and only for the host it was kept for.
    #[test]
    fn a_kept_key_sits_a_model_down_without_the_environment() {
        let dir = scratch("key-kept");
        let file = settings_in(&dir);
        let config = file.to_string_lossy().into_owned();
        let join = join_args(&["--profile", "keyed", "--config", &config]).unwrap();
        let none = |_: &str| None;
        let store = MemoryKeys::default();
        let pick = |store: &MemoryKeys| {
            choose(&join, &[], &none, store, AIProfile::default(), NOW).map(|c| c.label)
        };
        let refused = pick(&store).expect_err("no key anywhere");
        assert!(
            format!("{refused:#}").contains("TEST_OWN_KEY"),
            "{refused:#}"
        );
        let elsewhere = KeyEntry::new("TEST_OWN_KEY", "https://llm.example.org").unwrap();
        store.set(&elsewhere, "TEST-kept-0123456789abcdef").unwrap();
        assert!(pick(&store).is_err(), "kept for another host");
        let here = KeyEntry::new("TEST_OWN_KEY", "https://api.anthropic.com").unwrap();
        store.set(&here, "TEST-kept-0123456789abcdef").unwrap();
        assert!(pick(&store).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
