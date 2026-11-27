//! `baylee-seat deck`: an AI's own decks (`baylee_seat::deckbuild`,
//! `docs/llm-seat.md` §"Deck building").
//!
//! ```text
//! baylee-seat deck facts   --gateway <origin> [--out <file>]
//! baylee-seat deck check   --format <f> [--facts <file>] <deck file>
//! baylee-seat deck lend    --format <f> --dir <dir> [--facts <file>] [--seed <n>]
//! baylee-seat deck build   --format <f> --dir <dir> [--facts <file>] (--profile <p> | --mind <m>) …
//! baylee-seat deck improve --format <f> --dir <dir> [--facts <file>] (--profile <p> | --mind <m>) [--most 6] [--games <file>]…
//! ```

use anyhow::{Context as _, anyhow, bail};
use baylee_account::ranked::Format;
use baylee_client_core::llmseat::Provider;
use baylee_client_core::llmseat::keys::{self as llmseat_keys, KeyStore};
use baylee_client_core::say;
use baylee_seat::cli::{Brief, CliMind, Limits};
use baylee_seat::config::{self, Overrides, Paths, Plan};
use baylee_seat::deckbuild::brief::{self, Job};
use baylee_seat::deckbuild::check::{Checker, Rows};
use baylee_seat::deckbuild::facts::{self, Facts};
use baylee_seat::deckbuild::file;
use baylee_seat::deckbuild::history::{self, Record, Spend};
use baylee_seat::deckbuild::lend;
use baylee_seat::deckbuild::pool::Pool;
use baylee_seat::deckbuild::talk::{Chain, Chat, CliChat};
use baylee_seat::deckbuild::work::{self, Base, Task};
use baylee_seat::keys;
use baylee_seat::llm::deckchat::DeckChat;
use baylee_seat::llm::{self, AnswerMode, Spec};
use baylee_seat::spend::{self, Booked};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(clap::Args)]
pub struct DeckArgs {
    #[command(subcommand)]
    command: DeckCommand,
}

#[derive(clap::Subcommand)]
enum DeckCommand {
    /// Read a host's legality facts as a guest and keep them: the table a
    /// deck is checked against.
    Facts(FactsArgs),
    /// Hold a deck file to the host's check, and print its faults.
    Check(CheckArgs),
    /// Lend a deck made without a model, for the house and the trained net.
    Lend(LendArgs),
    /// Have a language model build a deck from the pool.
    Build(ModelArgs),
    /// Have a language model change a few cards of its newest deck.
    Improve(ImproveArgs),
}

#[derive(clap::Args)]
struct FactsArgs {
    /// The host's origin (`https://host`, or `http://` on this machine).
    #[arg(long)]
    gateway: String,
    /// Where they are kept [default: `facts.json` here].
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(clap::Args)]
struct Where {
    /// `standard`, `highlander` or `commander`.
    #[arg(long)]
    format: String,
    /// The AI's deck directory: its facts, and a directory per format.
    #[arg(long)]
    dir: PathBuf,
    /// The legality facts [default: `<dir>/facts.json`].
    #[arg(long)]
    facts: Option<PathBuf>,
}

#[derive(clap::Args)]
struct CheckArgs {
    /// `standard`, `highlander` or `commander`.
    #[arg(long)]
    format: String,
    /// The legality facts.
    #[arg(long)]
    facts: PathBuf,
    /// The deck file.
    deck: PathBuf,
}

#[derive(clap::Args)]
struct LendArgs {
    #[command(flatten)]
    at: Where,
    /// What decides every choice: the same seed and facts lend the same
    /// deck.
    #[arg(long, default_value_t = 1)]
    seed: u64,
}

#[derive(clap::Args)]
struct ModelArgs {
    #[command(flatten)]
    at: Where,
    /// The profile of the settings file that builds.
    #[arg(long)]
    profile: Option<String>,
    /// A model in the profile's place: `anthropic[:<model>]`,
    /// `openai:<model>`, `cli:<tool>[:<model>]`.
    #[arg(long)]
    mind: Option<String>,
    /// The settings file [default: `BAYLEE_SEAT_CONFIG`, else
    /// `llm-seat.json` in the client's config directory].
    #[arg(long)]
    config: Option<PathBuf>,
    /// The spend book [default: beside the settings file].
    #[arg(long)]
    ledger: Option<PathBuf>,
    /// How hard the model thinks, over the profile's.
    #[arg(long)]
    effort: Option<String>,
    /// A fallback to take over, in place of the profile's own; repeat for
    /// a chain.
    #[arg(long = "fallback-profile")]
    fallback_profiles: Vec<String>,
    /// No fallback at all.
    #[arg(long)]
    no_fallback: bool,
    /// The most replies the model gives.
    #[arg(long, default_value_t = 40)]
    rounds: u32,
    /// The longest one reply may take, in seconds.
    #[arg(long, default_value_t = 300)]
    reply_secs: u64,
}

#[derive(clap::Args)]
struct ImproveArgs {
    #[command(flatten)]
    model: ModelArgs,
    /// The most cards moved, out and in together.
    #[arg(long, default_value_t = 6)]
    most: u32,
    /// The most of the format's recent games told, newest last.
    #[arg(long, default_value_t = 20)]
    recent: usize,
    /// More about the games, as text files (transcript summaries, notes):
    /// told to the model as data.
    #[arg(long = "games")]
    games: Vec<PathBuf>,
}

/// The most of one games file told, in characters.
const GAMES_FILE_CHARS: usize = 6000;

/// The most of every games file together.
const GAMES_CHARS: usize = 24_000;

fn format_of(text: &str) -> anyhow::Result<Format> {
    text.parse().map_err(|_| {
        anyhow!("no ranked format is called «{text}»: standard, highlander or commander")
    })
}

fn load_facts(at: &Where) -> anyhow::Result<Facts> {
    let path = at.facts.clone().unwrap_or_else(|| facts::path_in(&at.dir));
    facts::load(&path).map_err(anyhow::Error::msg)
}

/// Runs `deck …`.
pub async fn run(
    args: &DeckArgs,
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
) -> anyhow::Result<()> {
    match &args.command {
        DeckCommand::Facts(facts) => fetch(facts).await,
        DeckCommand::Check(check) => check_file(check),
        DeckCommand::Lend(lend) => lend_deck(lend),
        DeckCommand::Build(build) => model_deck(build, None, env, keys).await,
        DeckCommand::Improve(improve) => model_deck(&improve.model, Some(improve), env, keys).await,
    }
}

async fn fetch(args: &FactsArgs) -> anyhow::Result<()> {
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from(facts::FILE));
    let kept = facts::load(&out).ok();
    let read = facts::fetch(&args.gateway, kept.as_ref()).await?;
    facts::save(&out, &read).with_context(|| format!("writing {}", out.display()))?;
    say!(
        "the legality facts of {} (version {}, {} cards, {} blocked) are in {}",
        read.gateway,
        read.version(),
        read.pool.cards.len(),
        read.blocked.len(),
        out.display()
    );
    Ok(())
}

fn check_file(args: &CheckArgs) -> anyhow::Result<()> {
    let format = format_of(&args.format)?;
    let facts = facts::load(&args.facts).map_err(anyhow::Error::msg)?;
    let rows = file::load(&args.deck, format).map_err(anyhow::Error::msg)?;
    let checker = Checker::new(&facts);
    let verdict = checker.check(format, &rows, &facts::now_utc());
    if verdict.passed() {
        say!(
            "«{}» passes the {} host's check",
            rows.name,
            format.as_str()
        );
        return Ok(());
    }
    for complaint in &verdict.complaints {
        say!("- {complaint}");
    }
    bail!("«{}» does not pass the host's check", rows.name)
}

fn lend_deck(args: &LendArgs) -> anyhow::Result<()> {
    let format = format_of(&args.at.format)?;
    let facts = load_facts(&args.at)?;
    let checker = Checker::new(&facts);
    let now = facts::now_utc();
    let pool = Pool::of(&checker, format, &now);
    let lent = lend::lend(&checker, &pool, format, args.seed, &now).map_err(anyhow::Error::msg)?;
    let mut record = Record {
        made: now.clone(),
        name: lent.rows.name.clone(),
        how: "lend".into(),
        seed: Some(args.seed),
        generated: Some(lent.generated.clone()),
        note: format!(
            "Lent without a model: the deck generator's {} from the {} pool of {} cards.",
            lent.generated,
            format.as_str(),
            pool.len()
        ),
        ..Record::default()
    };
    let saved = save_version(
        &args.at.dir,
        format,
        &lent.rows,
        &checker,
        &now,
        &mut record,
    )?;
    say!(
        "lent «{}» as version {} ({} tries passed over): {}",
        lent.rows.name,
        saved.number,
        lent.passed_over.len(),
        saved.deck.display()
    );
    Ok(())
}

/// Writes `rows` as the next version, after reading the file back and
/// holding what it reads to the check once more: the file is the deck the
/// runner offers.
fn save_version(
    dir: &Path,
    format: Format,
    rows: &Rows,
    checker: &Checker<'_>,
    now: &str,
    record: &mut Record,
) -> anyhow::Result<history::Version> {
    let text = file::text(rows, format).map_err(anyhow::Error::msg)?;
    let (back, named) = file::read(&text, &rows.name).map_err(anyhow::Error::msg)?;
    anyhow::ensure!(
        named.as_deref() == Some(format.as_str()),
        "the deck file names {named:?}, not {}",
        format.as_str()
    );
    let verdict = checker.check(format, &back, now);
    if !verdict.passed() {
        bail!(
            "the deck file reads back as a deck the check refuses: {}",
            verdict.complaints.join("; ")
        );
    }
    record.facts_version = checker.facts().version().to_string();
    record.facts_gateway.clone_from(&checker.facts().gateway);
    history::save(dir, format, &text, record).context("saving the version")
}

/// A model's chat and the reservation its spend is counted in.
struct Reached {
    chat: Box<dyn Chat>,
    booked: Option<Booked>,
}

/// The chain `args` names: the model, then its fallbacks, each reached and
/// reserved in its spend book.
fn chain_of(
    args: &ModelArgs,
    format: Format,
    job: Job,
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
) -> anyhow::Result<(Chain, Vec<Booked>)> {
    let paths = Paths::resolve(args.config.as_deref(), args.ledger.as_deref(), env);
    let file = paths.load().map_err(anyhow::Error::msg)?;
    let spec = args
        .mind
        .as_deref()
        .map(|text| {
            Spec::parse(text)
                .ok_or_else(|| {
                    anyhow!(
                        "--mind names a language model: anthropic, openai:<model> or cli:<tool>"
                    )
                })?
                .map_err(anyhow::Error::msg)
        })
        .transpose()?;
    let overrides = Overrides {
        effort: args.effort.clone(),
        ..Overrides::default()
    };
    let plan = config::plan(
        spec.as_ref(),
        file.as_ref(),
        &paths,
        args.profile.as_deref(),
        &overrides,
    )
    .map_err(anyhow::Error::msg)?
    .ok_or_else(|| anyhow!("name the model that builds: --profile <name> or --mind <model>"))?;
    let fallbacks = if args.no_fallback {
        Vec::new()
    } else if args.fallback_profiles.is_empty() {
        plan.fallback.clone()
    } else {
        args.fallback_profiles.clone()
    };
    let system = brief::system(format, job, args.rounds);
    let caps = file.as_ref().map_or_default(|file| file.caps);
    let book = paths.book(file.is_some());
    let book = book.as_ref();
    let mut reached = vec![reach(plan.clone(), &system, book, &caps, env, keys)?];
    for name in fallbacks {
        let fallback = config::plan(
            None,
            file.as_ref(),
            &paths,
            Some(&name),
            &Overrides::default(),
        )
        .map_err(anyhow::Error::msg)
        .and_then(|p| p.ok_or_else(|| anyhow!("no profile «{name}»")));
        match fallback.and_then(|p| reach(p, &system, book, &caps, env, keys)) {
            Ok(one) => reached.push(one),
            Err(why) => say!("the fallback {name} does not stand by: {why:#}"),
        }
    }
    let mut booked = Vec::new();
    let mut chats = Vec::new();
    for one in reached {
        if let Some(mut b) = one.booked {
            b.watch(one.chat.tally());
            say!("{}", b.sat_down());
            booked.push(b);
        }
        chats.push(one.chat);
    }
    Ok((Chain::new(chats), booked))
}

/// `plan`'s model, reached (its key from the environment or this
/// machine's store, its CLI's program found) and its work reserved.
fn reach(
    mut plan: Plan,
    system: &str,
    book: Option<&baylee_client_core::llmseat::ledger::Book>,
    caps: &baylee_client_core::llmseat::Caps,
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
) -> anyhow::Result<Reached> {
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
    let access = llm::check(&plan, &env).map_err(anyhow::Error::msg)?;
    let booked = book
        .map(|book| {
            spend::reserve(
                book,
                caps,
                &mut plan.settings,
                plan.profile.as_deref(),
                spend::now(),
            )
        })
        .transpose()
        .map_err(anyhow::Error::msg)?;
    let schema = brief::schema();
    let chat: Box<dyn Chat> = match access {
        llm::Access::Api(credentials) => {
            let answer = match plan.settings.answer {
                AnswerMode::Tools => {
                    "\n\nAnswer every reply by calling the tool `act` with that object."
                }
                AnswerMode::Json | AnswerMode::JsonSchema => {
                    "\n\nAnswer every reply with that one JSON object and nothing else."
                }
            };
            Box::new(DeckChat::new(
                plan.settings.clone(),
                credentials,
                format!("{system}{answer}"),
                schema,
            ))
        }
        llm::Access::Cli(launch) => {
            debug_assert_eq!(plan.settings.provider, Provider::Cli);
            let brief = Brief {
                system: format!(
                    "{system}\n\nYou have no tools. Answer every reply with that one JSON \
                     object and nothing else."
                ),
                schema,
                task: "Baylee deck building",
                answer: "the action JSON",
            };
            Box::new(CliChat::new(CliMind::with_brief(
                plan.settings.clone(),
                launch,
                Limits::default(),
                brief,
            )))
        }
    };
    Ok(Reached { chat, booked })
}

/// What `format`'s games in `dir` said, and the files named, as text for
/// the model.
fn games_text(
    dir: &Path,
    format: Format,
    recent: usize,
    files: &[PathBuf],
) -> anyhow::Result<String> {
    use std::fmt::Write as _;
    let games = history::games(dir, format);
    let from = games.len().saturating_sub(recent);
    let mut out = String::new();
    if games.is_empty() {
        out.push_str("No game of this format is recorded yet.\n");
    }
    for game in &games[from..] {
        let _ = writeln!(
            out,
            "{} · version {} · {}{}{}{}{}",
            game.at,
            game.version,
            game.result,
            game.turns.map_or_default(|t| format!(" in {t} turns")),
            game.delta.map_or_default(|d| format!(" · Elo {d:+}")),
            game.opponent
                .as_deref()
                .map_or_default(|o| format!(" · against {o}")),
            game.note.as_deref().map_or_default(|n| format!(" · {n}")),
        );
    }
    let mut told = 0;
    for path in files {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let take: String = text
            .chars()
            .take(GAMES_FILE_CHARS.min(GAMES_CHARS - told))
            .collect();
        told += take.chars().count();
        let name = path
            .file_name()
            .map_or_default(|n| n.to_string_lossy().into_owned());
        let _ = write!(out, "\n{name}:\n{}\n", take.replace("```", "'''"));
        if told >= GAMES_CHARS {
            break;
        }
    }
    Ok(out)
}

/// The newest version of `format`'s deck in `dir` to improve: its rows,
/// its note and its games, and its number.
fn base_of(dir: &Path, format: Format, improve: &ImproveArgs) -> anyhow::Result<(Base, u32)> {
    let version = history::versions(dir, format).pop().ok_or_else(|| {
        anyhow!(
            "there is no {} deck in {} to improve",
            format.as_str(),
            dir.display()
        )
    })?;
    let rows = file::load(&version.deck, format).map_err(anyhow::Error::msg)?;
    let record = version.read_record().map_err(anyhow::Error::msg)?;
    let games = games_text(dir, format, improve.recent, &improve.games)?;
    Ok((
        Base {
            rows,
            note: record.note,
            games,
        },
        version.number,
    ))
}

/// What every model of `chain` spent.
fn spent(chain: &Chain) -> Spend {
    let mut all = Spend::default();
    for tally in chain.tallies() {
        let tally = tally
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        all.calls += tally.calls;
        all.tokens += tally.spend_tokens();
        if let Some(usd) = tally.spend_usd() {
            *all.usd.get_or_insert(0.0) += usd;
        }
    }
    all
}

/// Settles every reservation now; one the book cannot take stays counted
/// in full.
fn settle(booked: &mut [Booked]) {
    for book in booked {
        if let Err(why) = book.settle(spend::now()) {
            say!("the reservation stays counted in full: {why}");
        }
    }
}

async fn model_deck(
    args: &ModelArgs,
    improve: Option<&ImproveArgs>,
    env: &dyn Fn(&str) -> Option<String>,
    keys: &dyn KeyStore,
) -> anyhow::Result<()> {
    let format = format_of(&args.at.format)?;
    let facts = load_facts(&args.at)?;
    let checker = Checker::new(&facts);
    let now = facts::now_utc();
    let pool = Pool::of(&checker, format, &now);
    anyhow::ensure!(
        !pool.is_empty(),
        "the {} pool is empty under the facts from {}",
        format.as_str(),
        facts.gateway
    );
    let (job, base, parent) = match improve {
        None => (Job::Build, None, None),
        Some(improve) => {
            let (base, number) = base_of(&args.at.dir, format, improve)?;
            (Job::Improve(improve.most), Some(base), Some(number))
        }
    };
    let (chain, mut booked) = chain_of(args, format, job, env, keys)?;
    let task = Task {
        format,
        job,
        rounds: args.rounds,
        reply: Duration::from_secs(args.reply_secs.max(1)),
        base,
        now: now.clone(),
    };
    let mut log = |line: &str| say!("{line}");
    let made = work::run(&chain, &checker, &pool, &task, &mut log).await;
    let handed: Vec<String> = chain
        .said
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    for line in handed {
        say!("{line}");
    }
    let made = match made {
        Ok(made) => made,
        Err(why) => {
            settle(&mut booked);
            bail!(why);
        }
    };
    let who = chain.current().map_or_default(Chat::who);
    let mut record = Record {
        made: now.clone(),
        name: made.rows.name.clone(),
        how: if improve.is_some() {
            "improve"
        } else {
            "build"
        }
        .into(),
        model: Some(who.model),
        profile: who.profile,
        effort: who.effort,
        parent,
        removed: made.removed.clone(),
        added: made.added.clone(),
        note: made.note.clone(),
        spend: Some(Spend {
            rounds: made.rounds,
            ..spent(&chain)
        }),
        ..Record::default()
    };
    let saved = save_version(
        &args.at.dir,
        format,
        &made.rows,
        &checker,
        &now,
        &mut record,
    );
    settle(&mut booked);
    let saved = saved?;
    say!(
        "«{}» is version {} of the {} deck, after {} replies: {}",
        made.rows.name,
        saved.number,
        format.as_str(),
        made.rounds,
        saved.deck.display()
    );
    if !made.removed.is_empty() || !made.added.is_empty() {
        say!(
            "out: {}; in: {}",
            made.removed.join(", "),
            made.added.join(", ")
        );
    }
    say!("its note: {}", made.note);
    Ok(())
}
