//! The ranked runner: an AI account that plays Ranked as a person does
//! (`docs/llm-seat.md` §"The ranked runner", `docs/proposals/ai-players.md`,
//! baylee-central `docs/ai-players.md`).
//!
//! From the outside in:
//!
//! - [`RunnerToken`]: the AI runner token an administrator issued for this
//!   machine, read from a credential file or the credential store;
//! - [`Player`]: the AI account, its runner sessions traded for the token,
//!   renewed and rolled over, Central's terms accepted;
//! - [`Shelf`]: its decks, files on this machine, one offered per format;
//! - [`run`]: its standing offers, renewed while it may take a game, and
//!   Central's work read every two seconds, each live match played as a
//!   task of its own through [`play_match`], as many at once as Central
//!   makes (`max_games`);
//! - [`play_match`]: one match, its seat played by the account's
//!   [`Pilot`] through a [`RankedLink`] (Central's relay: the hosts'
//!   terms, admission, deck, seat, socket, renewal, resume), and Central's
//!   result read back.
//!
//! What plays is a [`Pilot`], the one seam between the runner and the
//! mind: the house at a fair level ([`HousePilot`]) and the language models
//! here, the trained net beside them. A pilot that cannot go on after a
//! resume hands the seat to the declared house fallback.

pub mod chain;
pub mod cli;
mod deck;
mod game;
mod link;
mod pilot;
mod player;
#[cfg(test)]
mod tests;
mod token;

pub use deck::{Frozen, Shelf};
pub use game::{Match, MatchOptions, MatchReport, Receipt, play_match};
pub use link::{DeckRefused, LinkOptions, RankedLink, socket_url};
pub use pilot::{Flight, HousePilot, Pilot, Piloted, Seat, fly_bridge};
pub use player::{Player, ROLL_OVER, Session, resolve};
pub use token::RunnerToken;

use crate::deckbuild::facts;
use anyhow::anyhow;
use baylee_account::Error;
use baylee_account::ranked::Format;
use baylee_account::ranked::Id;
use baylee_account::ranked::ai::{self, Offer};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use uuid::Uuid;

/// A task that ends when this is dropped.
pub struct Aborting(pub(crate) JoinHandle<()>);

impl Drop for Aborting {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// What the runner is asked to do: play on, finish what runs (its offers
/// withdrawn), or leave now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stop {
    /// Offer and play.
    Run,
    /// Offer nothing more; play the running matches to their end.
    Drain,
    /// Leave the running matches now.
    Leave,
}

/// How the runner plays.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Matches to play before it stops; `None` plays on.
    pub games: Option<u32>,
    /// How often Central's work is read.
    pub poll: Duration,
    /// How often standing offers are renewed (Central keeps one 45 s).
    pub renew_offers: Duration,
    /// How often the deck directory and its facts are read again.
    pub reread_decks: Duration,
    /// The host whose legality facts the decks are checked against,
    /// fetched as a guest ([`facts::fetch`]) and kept beside them; `None`
    /// fetches none and uses what is kept there.
    pub facts_from: Option<String>,
    /// Legality facts the operator keeps, read and never written; beside
    /// fetched ones, the fresher of the two counts.
    pub facts_file: Option<std::path::PathBuf>,
    /// How often the facts are fetched again.
    pub refetch_facts: Duration,
    /// How each match is played.
    pub play: MatchOptions,
    /// Whether the account asks Central's carousel for a match against
    /// another AI while it offers and has a free slot (`--no-carousel`).
    pub carousel: bool,
    /// How often it asks the carousel per format while it may.
    pub carousel_every: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            games: None,
            poll: Duration::from_secs(2),
            renew_offers: Duration::from_secs(15),
            reread_decks: Duration::from_secs(60),
            facts_from: None,
            facts_file: None,
            refetch_facts: Duration::from_mins(30),
            play: MatchOptions::default(),
            carousel: true,
            carousel_every: Duration::from_secs(30),
        }
    }
}

/// Whether `pilot` may play the account Central's administrator set up as
/// `settings`: its kind and its key, exactly.
///
/// # Errors
/// Why not, in a sentence for the operator.
pub fn fits(pilot: &dyn Pilot, settings: &ai::Settings) -> Result<(), String> {
    if pilot.kind() != settings.badge.kind || pilot.profile() != settings.profile {
        return Err(format!(
            "Central has this account as {:?} `{}`, and this runner flies {:?} `{}`: \
             no offers until they agree",
            settings.badge.kind,
            settings.profile,
            pilot.kind(),
            pilot.profile()
        ));
    }
    Ok(())
}

/// Plays `player`'s ranked matches with `pilot` until `stop` says so or
/// [`Settings::games`] are over, `fallback` (the house) taking a seat the
/// pilot hands over. `each` is told how every match went.
///
/// While the account is switched on, its pilot fits Central's settings
/// ([`fits`]) and is ready, and it plays fewer matches than its
/// `max_games`, it offers the shelf's decks, renewed every
/// [`Settings::renew_offers`]; otherwise it withdraws them. Every live
/// match Central's work names is played, also one a runner before a
/// restart took (the link resumes it), as long as the shelf holds the
/// deck it was made with. When it stops, the offers are withdrawn; a drain
/// plays the running matches to their end, a leave drops them.
///
/// # Errors
/// None from Central's answers, which it logs and outlasts; only a task
/// that cannot be run.
pub async fn run(
    player: &Player,
    shelf: &mut Shelf,
    pilot: Arc<dyn Pilot>,
    fallback: Arc<dyn Pilot>,
    settings: &Settings,
    mut stop: watch::Receiver<Stop>,
    mut each: impl FnMut(anyhow::Result<MatchReport>) + Send,
) -> anyhow::Result<u32> {
    let _facts = settings
        .facts_from
        .clone()
        .zip(shelf.dir().map(facts::path_in))
        .map(|(origin, path)| fetch_facts(origin, path, settings.refetch_facts));
    let mut rounds = Rounds {
        player,
        shelf,
        pilot,
        fallback,
        settings,
        playing: tokio::task::JoinSet::new(),
        seen: HashSet::new(),
        done: 0,
        offered: None,
        reread: None,
        told: None,
        carousel_next: BTreeMap::new(),
        carousel_later: BTreeMap::new(),
    };
    loop {
        let stopping = *stop.borrow_and_update();
        let over = settings.games.is_some_and(|games| rounds.done >= games);
        if stopping == Stop::Leave
            || ((stopping == Stop::Drain || over) && rounds.playing.is_empty())
        {
            break;
        }
        rounds.round(stopping).await;
        tokio::select! {
            Some(joined) = rounds.playing.join_next(), if !rounds.playing.is_empty() => {
                rounds.done += 1;
                let report = joined.unwrap_or_else(|e| Err(anyhow!("the match's task ended: {e}")));
                rounds.after(&report);
                each(report);
            }
            _ = stop.changed() => {}
            () = tokio::time::sleep(settings.poll) => {}
        }
    }
    offer(player, &[], &mut rounds.offered).await;
    Ok(rounds.done)
}

/// What [`run`] keeps from one round to the next.
struct Rounds<'a> {
    player: &'a Player,
    shelf: &'a mut Shelf,
    pilot: Arc<dyn Pilot>,
    fallback: Arc<dyn Pilot>,
    settings: &'a Settings,
    playing: tokio::task::JoinSet<anyhow::Result<MatchReport>>,
    /// Every match started, so a match Central still names is not played
    /// twice.
    seen: HashSet<Id>,
    done: u32,
    /// The offers last made, and when.
    offered: Option<(Vec<Offer>, Instant)>,
    /// When the decks were last read.
    reread: Option<Instant>,
    /// Why the account makes no offers, as last told.
    told: Option<String>,
    /// When the carousel may be asked again, per format.
    carousel_next: BTreeMap<Format, Instant>,
    /// The carousel's `later` answers in a row, per format.
    carousel_later: BTreeMap<Format, u32>,
}

impl Rounds<'_> {
    /// One round: the decks read again when it is time, Central's work
    /// read, its new matches started, and the offers renewed or withdrawn.
    async fn round(&mut self, stopping: Stop) {
        if self
            .reread
            .is_none_or(|at| at.elapsed() >= self.settings.reread_decks)
        {
            let kept = self.facts();
            self.shelf
                .refresh(kept.as_ref(), |line| tracing::info!("{line}"));
            self.reread = Some(Instant::now());
        }
        match self
            .player
            .call(|account| account.ai(ai::Request::Work))
            .await
        {
            Ok(ai::Response::Work {
                settings,
                assignments,
                ..
            }) => {
                for assignment in assignments {
                    self.start(assignment);
                }
                let why_not = self.why_not(stopping, &settings);
                let desired = if why_not.is_none() {
                    self.shelf.offers()
                } else {
                    Vec::new()
                };
                self.tell(why_not);
                let due = self.offered.as_ref().is_none_or(|(last, at)| {
                    *last != desired
                        || (!desired.is_empty() && at.elapsed() >= self.settings.renew_offers)
                });
                if due {
                    offer(self.player, &desired, &mut self.offered).await;
                }
                let offering = self
                    .offered
                    .as_ref()
                    .is_some_and(|(last, _)| *last == desired);
                if self.settings.carousel && !desired.is_empty() && offering {
                    self.carousel(&desired, settings.max_games).await;
                }
            }
            Ok(_) => tracing::warn!("Central answered the work with something else"),
            Err(Error::Rejected) => match self.player.roll_over().await {
                Ok(true) => tracing::warn!(
                    "Central refused the work on the newest session; it goes on with a fresh one"
                ),
                Ok(false) => tracing::debug!("Central refused the work; a fresh session waits"),
                Err(e) => tracing::warn!(
                    error = %format!("{e:#}"),
                    "Central refused the work, and no fresh runner session yet"
                ),
            },
            Err(e) => tracing::info!(error = %e, "Central could not say the work"),
        }
    }

    /// Plays `assignment` as a task of its own, unless it already runs, or
    /// neither the shelf nor any version in the deck directory has the deck
    /// it was made with (a runner that restarted finds an older version
    /// there).
    fn start(&mut self, assignment: baylee_account::ranked::Assignment) {
        if self.seen.contains(&assignment.match_id) {
            return;
        }
        let id = Uuid::from_bytes(assignment.match_id);
        let deck = self
            .shelf
            .by_digest(&assignment.deck)
            .or_else(|| self.shelf.recall(assignment.format, &assignment.deck));
        self.seen.insert(assignment.match_id);
        let Some(deck) = deck else {
            tracing::warn!(r#match = %id, "a match with a deck this runner does not hold; it cannot be played here");
            return;
        };
        tracing::info!(r#match = %id, format = assignment.format.as_str(), "playing a match");
        let (player, pilot, fallback, options) = (
            self.player.clone(),
            Arc::clone(&self.pilot),
            Arc::clone(&self.fallback),
            self.settings.play.clone(),
        );
        self.playing.spawn(async move {
            play_match(
                &player,
                Match { assignment, deck },
                &*pilot,
                &*fallback,
                &options,
            )
            .await
        });
    }

    /// Why the account makes no offers now, or `None` where it does.
    fn why_not(&self, stopping: Stop, settings: &ai::Settings) -> Option<String> {
        let running = self.playing.len();
        let asked_for = self.settings.games.is_some_and(|games| {
            u64::from(self.done) + u64::try_from(running).unwrap_or(u64::MAX) >= u64::from(games)
        });
        if stopping != Stop::Run {
            Some("stopping: no more offers".to_owned())
        } else if asked_for {
            Some("as many matches as asked for: no more offers".to_owned())
        } else if !settings.enabled {
            Some("Central has this AI switched off: no offers".to_owned())
        } else if let Err(why) = fits(&*self.pilot, settings) {
            Some(why)
        } else if let Err(why) = self.pilot.ready() {
            Some(format!("the pilot cannot take a game: {why}"))
        } else if running >= usize::from(settings.max_games) {
            Some(format!(
                "playing {running} of {} at once",
                settings.max_games
            ))
        } else if self.shelf.offers().is_empty() {
            Some("no deck to offer".to_owned())
        } else {
            None
        }
    }

    /// Asks Central's carousel for a match against another AI in each
    /// offered format whose turn has come, while a slot is free under
    /// `max_games` (baylee-central `docs/ai-players.md` §7). A seat is
    /// played as any match; a refusal says when to ask again: `off` (the
    /// carousel is off for the account) in ten minutes, `offer` once the
    /// offers are renewed, `tomorrow` after the next midnight UTC, `later`
    /// after a jittered wait from 30 s that doubles with each one in a row,
    /// at most ten minutes. Central decides everything else.
    async fn carousel(&mut self, offers: &[Offer], max_games: u8) {
        for offer in offers {
            if self.playing.len() >= usize::from(max_games) {
                return;
            }
            let format = offer.format;
            if self
                .carousel_next
                .get(&format)
                .is_some_and(|at| Instant::now() < *at)
            {
                continue;
            }
            let answer = self
                .player
                .call(move |account| account.ai_carousel(format))
                .await;
            let wait = match answer {
                Ok(Ok(assignment)) => {
                    tracing::info!(
                        format = format.as_str(),
                        "the carousel seated a match against an AI"
                    );
                    self.carousel_later.remove(&format);
                    self.start(assignment);
                    self.settings.carousel_every
                }
                Ok(Err(reason)) => {
                    tracing::debug!(format = format.as_str(), reason = ?reason, "the carousel seated no match");
                    self.unseated(format, reason)
                }
                Err(e) => {
                    tracing::info!(error = %e, "the carousel could not be asked");
                    Duration::from_mins(1)
                }
            };
            self.carousel_next.insert(format, Instant::now() + wait);
        }
    }

    /// How long to wait before the carousel is asked again in `format`
    /// after it seated no match for `reason`.
    fn unseated(&mut self, format: Format, reason: ai::Unseated) -> Duration {
        match reason {
            ai::Unseated::Off => {
                self.carousel_later.remove(&format);
                Duration::from_mins(10)
            }
            ai::Unseated::Offer => {
                // Offered anew at the next round, then asked again.
                self.offered = None;
                self.settings.poll
            }
            ai::Unseated::Tomorrow => {
                self.carousel_later.remove(&format);
                until_midnight_utc()
            }
            ai::Unseated::Later => {
                let times = self.carousel_later.entry(format).or_insert(0);
                *times = times.saturating_add(1);
                jittered(
                    Duration::from_secs(30)
                        .saturating_mul(1 << (*times - 1).min(5))
                        .min(Duration::from_mins(10)),
                )
            }
        }
    }

    /// What a finished match leaves: its game in the deck's chronicle
    /// (`<decks>/<format>/games.jsonl`, which the deck builder reads), and
    /// a deck the host would not seat offered no more.
    fn after(&mut self, report: &anyhow::Result<MatchReport>) {
        match report {
            Ok(m) => {
                let (Some(dir), Some(version)) = (self.shelf.dir(), self.shelf.version_of(&m.deck))
                else {
                    return;
                };
                let result = match m.receipt.map(|r| r.won()) {
                    Some(Some(true)) => "won",
                    Some(Some(false)) => "lost",
                    Some(None) => "draw",
                    None => "unrated",
                };
                let mut notes = Vec::new();
                if m.receipt.is_some_and(|r| r.inferred) {
                    notes.push("result inferred from the account's public standing".to_owned());
                }
                if let Some(why) = &m.handed_over {
                    notes.push(format!("the house took the seat over: {why}"));
                }
                let game = crate::deckbuild::history::Game {
                    at: facts::now_utc(),
                    version,
                    result: result.into(),
                    turns: Some(m.played.stats.turns),
                    elo: m.receipt.map(|r| r.elo),
                    delta: m.receipt.and_then(|r| r.delta),
                    opponent: m.opponent.clone(),
                    note: (!notes.is_empty()).then(|| notes.join("; ")),
                };
                if let Err(e) = crate::deckbuild::history::record_game(dir, m.format, &game) {
                    tracing::warn!(error = %e, "the match was not written to the deck's chronicle");
                }
            }
            Err(e) => {
                if let Some(refused) = e.chain().find_map(|c| c.downcast_ref::<DeckRefused>()) {
                    tracing::warn!("{refused}");
                    self.shelf.refuse(refused.digest);
                    self.reread = None;
                }
            }
        }
    }

    /// The legality facts the decks are checked against: the fresher of
    /// the operator's file and the fetched ones beside the decks. Best
    /// effort: with none, every deck that freezes is offered, and the
    /// host's own check at seating is what stands.
    fn facts(&self) -> Option<facts::Facts> {
        let read = |path: std::path::PathBuf| {
            facts::load(&path)
                .map_err(
                    |why| tracing::debug!(why = %why, path = %path.display(), "no legality facts"),
                )
                .ok()
        };
        let given = self.settings.facts_file.clone().and_then(read);
        let fetched = self.shelf.dir().map(facts::path_in).and_then(read);
        match (given, fetched) {
            (Some(given), Some(fetched)) => Some(if fetched.fetched > given.fetched {
                fetched
            } else {
                given
            }),
            (given, fetched) => given.or(fetched),
        }
    }

    /// Says why the account makes no offers, when that changed.
    fn tell(&mut self, why: Option<String>) {
        if why == self.told {
            return;
        }
        match &why {
            Some(why) => tracing::warn!("{why}"),
            None if self.told.is_some() => tracing::info!("offering again"),
            None => {}
        }
        self.told = why;
    }
}

/// Replaces the account's offers with `offers` (none withdraws them),
/// accepting Central's terms first where they changed.
async fn offer(player: &Player, offers: &[Offer], offered: &mut Option<(Vec<Offer>, Instant)>) {
    let sent = offers.to_vec();
    let answer = player
        .call(move |account| account.ai(ai::Request::Offer { offers: sent }))
        .await;
    match answer {
        Ok(_) => *offered = Some((offers.to_vec(), Instant::now())),
        Err(Error::TermsRequired) => {
            if let Err(e) = player.accept_terms().await {
                tracing::warn!(error = %format!("{e:#}"), "no offers until the terms are accepted");
            }
        }
        Err(e) => tracing::info!(error = %e, "the offers were not renewed"),
    }
}

/// Fetches the host's legality facts at `origin` now and every `every`,
/// saving them at `path`; a fetch that fails keeps what is saved and says
/// why. Runs until dropped.
fn fetch_facts(origin: String, path: std::path::PathBuf, every: Duration) -> Aborting {
    Aborting(tokio::spawn(async move {
        loop {
            let kept = facts::load(&path).ok();
            match facts::fetch(&origin, kept.as_ref()).await {
                Ok(fresh) => {
                    if kept.as_ref().is_none_or(|k| k.version() != fresh.version()) {
                        tracing::info!(version = fresh.version(), "the host's legality facts");
                    }
                    if let Err(e) = facts::save(&path, &fresh) {
                        tracing::warn!(error = %e, path = %path.display(), "the legality facts were not saved");
                    }
                }
                Err(e) => tracing::warn!(
                    error = %format!("{e:#}"),
                    "the host's legality facts were not fetched; the ones kept stand"
                ),
            }
            tokio::time::sleep(every).await;
        }
    }))
}

/// From now until a moment past the next midnight UTC.
fn until_midnight_utc() -> Duration {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Duration::from_secs(86_400 - now % 86_400 + 5)
}

/// `wait`, between three quarters of it and a quarter more, so that the
/// runners of a machine do not ask in step.
fn jittered(wait: Duration) -> Duration {
    let spread = Uuid::now_v7().as_u128() % 501;
    let factor = 750 + u32::try_from(spread).unwrap_or(0);
    wait.saturating_mul(factor) / 1000
}
