//! A language model as a ranked pilot (`docs/llm-seat.md` §"The ranked
//! runner", §"A fallback profile"; the owner's decisions of 26.11.2026 in
//! `docs/proposals/ai-players.md` §"Speed and stand-ins inside the AI").
//!
//! A language-model AI account is one model at one effort, reached on a
//! chain of access paths: the settings file's profile (`--profile`) first,
//! then the profiles its `fallback` names, in order: a subscription's CLI,
//! another subscription, the provider's API. Every path must play the same
//! model at the same effort ([`same_model`]): a chain that would cross to
//! another model is refused before the runner offers anything, since the
//! account's name and rating are the model's.
//!
//! Before a game, the pilot is ready while some path is not held back: a
//! path that was rate limited stays out of new games until its limit lifts
//! (as the provider said, else [`LIMIT_HOLD`]), one that failed for
//! [`FAILED_HOLD`]; with every path held back, the account makes no offer.
//! A game starts on the first path not held back, and the rest stand by in
//! order; mid-game, the bridge hands the game down the chain at once on a
//! limit or a failure, each path declaring its model to the table before
//! its first answer. Past the last path the declared house plays the rest
//! of the game (a house fallback at the chain's end, declared as the house,
//! so the record says what played): a ranked seat never leaves its table
//! because its model is gone.
//!
//! The spend book counts every path's game as `join` does. Tonight's
//! owner's decision is no caps: the file's day and month caps are the
//! mechanism, absent by default, and a profile that names no per-game
//! budget plays without one ([`UNLIMITED_USD`], [`UNLIMITED_TOKENS`],
//! [`UNLIMITED_CALLS`]) instead of the bridge's defaults.

use super::{Fallen, Join, Reachable, fallback_join, reach, standing_by};
use anyhow::{Context as _, anyhow, bail};
use baylee_account::ranked::ai::AiKind;
use baylee_ai::AIProfile;
use baylee_client_core::llmseat::keys::KeyStore;
use baylee_client_core::say;
use baylee_protocol::v1::{SeatMind, seat_mind};
use baylee_seat::bridge;
use baylee_seat::config::Paths;
use baylee_seat::ranked::chain::{
    FAILED_HOLD, Held, hold_back, holding, house_at_the_end, same_model,
};
use baylee_seat::ranked::{Flight, Pilot, Piloted, Seat};
use baylee_seat::seat::{BridgeConfig, Labelled};
use baylee_seat::spend;
use baylee_seat::{Mind, SeatCore, declare};
use clap::Parser as _;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// A game's budget in dollars where the profile names none.
pub const UNLIMITED_USD: f64 = 1_000_000.0;
/// A game's budget in tokens where the profile names none.
pub const UNLIMITED_TOKENS: u64 = 1_000_000_000_000;
/// A game's most calls where the profile names none.
pub const UNLIMITED_CALLS: u64 = 1_000_000_000;

/// Where the pilot's settings come from.
pub struct Setup {
    /// The settings file's profile the account plays: the chain's first
    /// path.
    pub profile: String,
    /// The settings file.
    pub config: Option<PathBuf>,
    /// The spend book.
    pub ledger: Option<PathBuf>,
    /// Where the model's reports file goes (beside the transcripts).
    pub transcripts: Option<PathBuf>,
    /// The house that plays past the chain's end, and answers what the
    /// model declines.
    pub house_level: String,
}

/// One access path: its profile, the command line it plays under, and the
/// model it declares.
struct Path {
    profile: String,
    join: Join,
    declared: SeatMind,
}

/// A language model flying an AI account's seats.
pub struct LlmPilot {
    paths: Vec<Path>,
    house_level: String,
    house: AIProfile,
    keys: Arc<dyn KeyStore>,
    held: Held,
    /// Failures in a row before a game, per path.
    failures: Arc<Mutex<BTreeMap<String, u32>>>,
}

impl std::fmt::Debug for LlmPilot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmPilot")
            .field(
                "paths",
                &self.paths.iter().map(|p| &p.profile).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

/// The environment the paths read their keys from.
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `join`'s command line, as `baylee-seat join` would parse it: the paths
/// play exactly what that command plays, through the same code.
fn join_line(args: &[OsString]) -> anyhow::Result<Join> {
    #[derive(clap::Parser)]
    struct Line {
        #[command(flatten)]
        join: Join,
    }
    Line::try_parse_from(args)
        .map(|line| line.join)
        .map_err(|e| anyhow!("{e}"))
}

impl LlmPilot {
    /// The pilot of `setup.profile` and the chain its `fallback` names,
    /// each path reached now (its key or its program found), every one the
    /// same model at the same effort.
    ///
    /// # Errors
    /// No settings file or no such profile, a path that cannot be reached,
    /// a chain that crosses models or efforts, or a profile Central cannot
    /// name (`model:<profile>`).
    pub fn new(setup: &Setup, keys: Arc<dyn KeyStore>) -> anyhow::Result<Self> {
        let house = baylee_seat::house::level(&setup.house_level).map_err(|why| anyhow!(why))?;
        let mut line: Vec<OsString> = vec![
            "ranked".into(),
            "ranked".into(),
            "--profile".into(),
            setup.profile.clone().into(),
        ];
        for (flag, path) in [
            ("--config", &setup.config),
            ("--ledger", &setup.ledger),
            ("--transcripts", &setup.transcripts),
        ] {
            if let Some(path) = path {
                line.push(flag.into());
                line.push(path.clone().into());
            }
        }
        let base = join_line(&line)?;
        let file = Paths::resolve(base.config.as_deref(), base.ledger.as_deref(), &env)
            .load()
            .map_err(anyhow::Error::msg)?
            .context("a language model's account plays from a settings file (--config)")?;
        let profile = file
            .profiles
            .get(&setup.profile)
            .with_context(|| format!("the settings file has no profile «{}»", setup.profile))?;
        let key = format!("model:{}", setup.profile);
        if !baylee_account::ranked::ai::ranked_profile(&key) {
            bail!(
                "Central names a model's account `model:<profile>`, lowercase letters, digits, \
                 `_`, `.` and `-`: «{}» cannot be one",
                setup.profile
            );
        }
        let names: Vec<String> = std::iter::once(setup.profile.clone())
            .chain(profile.fallback.iter().cloned())
            .collect();
        let mut paths: Vec<Path> = Vec::with_capacity(names.len());
        for (at, name) in names.iter().enumerate() {
            let own = file
                .profiles
                .get(name)
                .with_context(|| format!("the settings file has no profile «{name}»"))?;
            let plain = if at == 0 {
                base.clone()
            } else {
                Join {
                    reports_dir: base.reports_dir.clone(),
                    ..fallback_join(&base, at - 1, name)
                }
            };
            let join = unlimited(plain, own, &*keys);
            let reached = reach(&join, None, &[], &env, &*keys)
                .with_context(|| format!("the access path «{name}»"))?
                .with_context(|| format!("the settings file names no profile «{name}»"))?;
            let declared = declare::llm(&reached.plan.settings);
            // One path alone plays as its profile says; a second path must
            // name the same model at the same effort as the first, both
            // named (`same_model`), or the chain could cross models.
            if let Some(first) = paths.first() {
                same_model(&first.declared, &declared).map_err(|why| {
                    anyhow!(
                        "the access path «{name}» plays another model than «{}»: {why}; an AI \
                         account's chain reaches one model at one effort",
                        first.profile
                    )
                })?;
            } else if !matches!(
                seat_mind::Kind::try_from(declared.kind),
                Ok(seat_mind::Kind::LlmApi | seat_mind::Kind::LlmCli)
            ) {
                bail!("the profile «{name}» plays no language model");
            }
            if paths.iter().any(|p| p.profile == *name) {
                bail!("the access path «{name}» is named twice in the chain");
            }
            paths.push(Path {
                profile: name.clone(),
                join,
                declared,
            });
        }
        say!("the model's access paths, in order: {}", names.join(" → "));
        Ok(Self {
            paths,
            house_level: setup.house_level.clone(),
            house,
            keys,
            held: Held::default(),
            failures: Arc::default(),
        })
    }

    /// The paths not held back at `now`, in the chain's order; the holds
    /// that lifted are dropped.
    fn live(&self, now: Instant) -> Vec<&Path> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.retain(|_, until| *until > now);
        self.paths
            .iter()
            .filter(|p| !held.contains_key(&p.profile))
            .collect()
    }

    /// Holds `profile` back from new games for `hold`.
    fn hold(&self, profile: &str, hold: Duration, why: &str) {
        hold_back(&self.held, profile, hold, why);
    }

    /// The chain's first live path, reached, reserved and checked for a
    /// game: the mind, its booking, what it declares and how long it may
    /// think. A path that cannot be reached or reserved, or whose free
    /// check fails (a model's entry, a CLI's login: `Mind::check`, no
    /// game's call), is held back and the next tried, each failure in a
    /// row holding it twice as long ([`Self::failed`]). With the rest of
    /// the live paths behind it.
    async fn first<'a>(&self, live: &'a [&'a Path]) -> Option<(Started, &'a [&'a Path])> {
        for (at, path) in live.iter().enumerate() {
            let why = match start(path, &*self.keys) {
                Ok(started) => match started.mind.check().await {
                    Ok(()) => {
                        self.failures
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .remove(&path.profile);
                        return Some((started, &live[at + 1..]));
                    }
                    Err(why) => why,
                },
                Err(why) => why,
            };
            self.failed(&path.profile, &why);
        }
        None
    }

    /// `profile` failed before a game: held back for [`FAILED_HOLD`],
    /// doubled for each failure in a row, at most [`MOST_HOLD`], so a path
    /// that stays dead is asked rarely.
    fn failed(&self, profile: &str, why: &str) {
        let times = {
            let mut failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
            let times = failures.entry(profile.to_owned()).or_insert(0);
            *times = times.saturating_add(1);
            *times
        };
        let hold = FAILED_HOLD
            .saturating_mul(1 << (times - 1).min(5))
            .min(MOST_HOLD);
        self.hold(profile, hold, why);
    }
}

/// The longest a path that keeps failing before its games is held back.
pub const MOST_HOLD: Duration = Duration::from_hours(4);

/// `join` under the owner's no-caps decision: each per-game budget the
/// profile `own` does not name is lifted. A model without a price takes
/// no dollar budget, so that one is tried, and dropped where it is refused.
fn unlimited(join: Join, own: &baylee_client_core::llmseat::Profile, keys: &dyn KeyStore) -> Join {
    let open = Join {
        spend_tokens: join
            .spend_tokens
            .or(own.game_tokens.is_none().then_some(UNLIMITED_TOKENS)),
        spend_calls: join
            .spend_calls
            .or(own.game_calls.is_none().then_some(UNLIMITED_CALLS)),
        ..join
    };
    let dollars = Join {
        spend_usd: open
            .spend_usd
            .or(own.game_usd.is_none().then_some(UNLIMITED_USD)),
        ..open.clone()
    };
    match reach(&dollars, None, &[], &env, keys) {
        Ok(_) => dollars,
        Err(_) => open,
    }
}

/// A path's game, started: its mind, reservation and declaration.
struct Started {
    profile: String,
    mind: Arc<dyn Mind>,
    booked: Option<spend::Booked>,
    declared: SeatMind,
    think: Duration,
}

/// Reaches `path` and reserves its game in its book.
fn start(path: &Path, keys: &dyn KeyStore) -> Result<Started, String> {
    let Reachable {
        mut plan,
        access,
        book,
        caps,
    } = reach(&path.join, None, &[], &env, keys)
        .map_err(|e| format!("{e:#}"))?
        .ok_or_else(|| format!("the settings file names no profile «{}»", path.profile))?;
    let mut booked = book
        .map(|book| {
            spend::reserve(
                &book,
                &caps,
                &mut plan.settings,
                plan.profile.as_deref(),
                spend::now(),
            )
        })
        .transpose()?;
    if let Some(booked) = &booked {
        say!("{}", booked.sat_down());
    }
    let declared = declare::llm(&plan.settings);
    let built = access.build(&plan);
    if let Some(booked) = &mut booked {
        booked.watch(Arc::clone(&built.tally));
    }
    Ok(Started {
        profile: path.profile.clone(),
        mind: built.mind,
        booked,
        declared,
        think: Duration::from_secs(plan.think_secs),
    })
}

impl Pilot for LlmPilot {
    fn declared(&self) -> SeatMind {
        self.paths[0].declared.clone()
    }

    fn kind(&self) -> AiKind {
        AiKind::Model
    }

    fn profile(&self) -> String {
        format!("model:{}", self.paths[0].profile)
    }

    fn ready(&self) -> Result<(), String> {
        let now = Instant::now();
        if self.live(now).is_empty() {
            let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            let list: Vec<String> = held
                .iter()
                .map(|(p, until)| {
                    format!(
                        "{p} for {} min",
                        until.saturating_duration_since(now).as_secs().div_ceil(60)
                    )
                })
                .collect();
            return Err(format!(
                "every access path of the model is held back ({})",
                list.join(", ")
            ));
        }
        Ok(())
    }

    fn fly<'a>(&'a self, seat: Seat<'a>) -> Flight<'a> {
        Box::pin(async move {
            let live = self.live(Instant::now());
            let Some((started, rest)) = self.first(&live).await else {
                return Ok(Piloted::HandOver(
                    "no access path of the model could take the game".into(),
                ));
            };
            let Started {
                profile: first,
                mind,
                booked,
                declared,
                think,
            } = started;
            let fallen: Fallen = Arc::default();
            let mut chain = Vec::with_capacity(rest.len() + 1);
            for path in rest {
                match reach(&path.join, None, &[], &env, &*self.keys) {
                    Ok(Some(reached)) => chain.push(holding(
                        standing_by(&path.profile, reached, &fallen),
                        first.clone(),
                        Arc::clone(&self.held),
                    )),
                    Ok(None) => {}
                    Err(e) => self.hold(&path.profile, FAILED_HOLD, &format!("{e:#}")),
                }
            }
            chain.push(holding(
                house_at_the_end(self.house_level.clone(), self.house),
                first.clone(),
                Arc::clone(&self.held),
            ));
            let config = BridgeConfig {
                think,
                house: self.house,
                labelled: Labelled::Account,
                ..BridgeConfig::default()
            };
            let core = SeatCore::new(config, seat.deck.bridge.list.clone(), mind.disclosure())
                .with_mind(declared);
            let played = bridge::play_swapping(
                seat.link,
                core,
                mind,
                seat.transcript,
                seat.play,
                None,
                chain,
            )
            .await;
            if let Ok(played) = &played
                && !played.fallbacks.is_empty()
            {
                seat.transcript
                    .write_value(&serde_json::json!({ "fell_back": played.fallbacks }));
            }
            // Each reservation settles with what it spent, the first path's
            // and every fallback's that took over.
            if let Some(mut booked) = booked
                && let Err(why) = booked.settle(spend::now())
            {
                say!("the game's reservation stays counted in full: {why}");
            }
            drop(fallen);
            played.map(|played| Piloted::Played(Box::new(played)))
        })
    }
}

#[cfg(test)]
mod tests;
