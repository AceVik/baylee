//! The ranked runner: an AI account that plays Ranked as a person does
//! (`docs/llm-seat.md` §"The ranked runner", `docs/proposals/ai-players.md`).
//!
//! Per account, from the outside in:
//!
//! - [`Player`]: the account, signed in to Baylee Central as the native
//!   client signs in, its label checked, its session kept alive;
//! - [`Frozen`]: one of its private decks, frozen for ranked play;
//! - [`Assignments`]: where its matches come from ([`Queue`]: join, wait,
//!   take the assignment, or resume the one Central already holds);
//! - [`run`]: every match the source names, each as its own task, through
//!   [`play_match`];
//! - [`play_match`]: one match, its seat played by the bridge through a
//!   [`RankedLink`] (Central's relay: admission, deck, seat, socket,
//!   renewal, resume) and its receipt read back.
//!
//! What plays is a [`Pilot`]: any [`crate::Mind`], built afresh for each
//! match (the house at a fair level today, a language model later), and
//! what the seat declares it is.
//!
//! The pieces are cut so that what Central does not do yet plugs in where
//! it lands: a match comes as a [`Match`] (its assignment and deck) from an
//! [`Assignments`] source, the person's queue today and Central's standing
//! offers and live matches for AI accounts tomorrow (baylee-central
//! `docs/ai-players.md` §4), and [`run`] plays as many at once as the
//! source names.

mod deck;
mod game;
mod link;
mod player;
mod source;
#[cfg(test)]
mod tests;

pub use deck::Frozen;
pub use game::{Match, MatchOptions, MatchReport, Receipt, play_match};
pub use link::{LinkOptions, RankedLink, socket_url};
pub use player::{AI_NAME_PREFIX, Credentials, Player, labelled};
pub use source::{Assignments, Queue};

use crate::mind::Mind;
use anyhow::anyhow;
use baylee_account::ranked::Id;
use baylee_ai::AIProfile;
use baylee_protocol::v1::SeatMind;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// A task that ends when this is dropped.
pub struct Aborting(pub(crate) JoinHandle<()>);

impl Drop for Aborting {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// What plays the account's seat.
#[derive(Clone)]
pub struct Pilot {
    /// A fresh mind for each match.
    pub make: Arc<dyn Fn() -> Arc<dyn Mind> + Send + Sync>,
    /// What the seat says plays it, for the game's record.
    pub declared: SeatMind,
    /// The house that answers what the mind does not.
    pub house: AIProfile,
    /// The longest one answer may take.
    pub think: Duration,
    /// What kind of mind plays: the label the account must carry.
    pub disclosure: crate::Disclosure,
}

impl std::fmt::Debug for Pilot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pilot")
            .field("declared", &self.declared)
            .field("think", &self.think)
            .finish_non_exhaustive()
    }
}

impl Pilot {
    /// The house at a fair `level` (`expert`); never `seer`, which reads
    /// hidden cards a seat is never sent ([`crate::house::level`]).
    ///
    /// # Errors
    /// A level the house has not, or one that peeks.
    pub fn house(level: &str) -> anyhow::Result<Self> {
        let profile = crate::house::level(level).map_err(|why| anyhow!(why))?;
        Ok(Self {
            make: Arc::new(move || Arc::new(crate::HouseMind::new(profile)) as Arc<dyn Mind>),
            declared: crate::declare::house(level),
            house: profile,
            think: Duration::from_secs(30),
            disclosure: crate::Disclosure::House,
        })
    }
}

/// How the runner plays.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Matches to play before it stops; `None` plays on.
    pub games: Option<u32>,
    /// How often the source is asked for the account's live matches.
    pub poll: Duration,
    /// How long one Central session may serve before the runner signs in
    /// again at the next moment it plays no match (Central ends a session
    /// twelve hours after its sign-in).
    pub session_age: Duration,
    /// How each match is played.
    pub play: MatchOptions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            games: None,
            poll: Duration::from_secs(2),
            session_age: Duration::from_hours(8),
            play: MatchOptions::default(),
        }
    }
}

/// Plays the matches `source` names for `player` until
/// [`Settings::games`] are over, each as its own task, as many at once as
/// the source names: a match Central already holds for the account first
/// (a runner that restarted resumes it). `each` is told how every match
/// went. Asks the source for more only while it wants more.
///
/// # Errors
/// When the source fails (a refused search, one that waited too long). A
/// match that fails is told to `each` as an error and the runner goes on.
pub async fn run(
    player: &Player,
    source: &mut impl Assignments,
    settings: &Settings,
    pilot: &Pilot,
    mut each: impl FnMut(anyhow::Result<MatchReport>) + Send,
) -> anyhow::Result<u32> {
    let mut playing = tokio::task::JoinSet::new();
    let mut seen: HashSet<Id> = HashSet::new();
    let mut done = 0;
    let wanted = |done: u32, running: usize| {
        settings.games.is_none_or(|games| {
            u64::from(done) + u64::try_from(running).unwrap_or(u64::MAX) < u64::from(games)
        })
    };
    while settings.games.is_none_or(|games| done < games) {
        if playing.is_empty() && player.age() >= settings.session_age {
            player.sign_in_again().await?;
        }
        if wanted(done, playing.len()) {
            for game in source.live().await? {
                if !wanted(done, playing.len()) || !seen.insert(game.assignment.match_id) {
                    continue;
                }
                tracing::info!(r#match = %Uuid::from_bytes(game.assignment.match_id), "playing a match");
                let (player, pilot, options) =
                    (player.clone(), pilot.clone(), settings.play.clone());
                playing.spawn(async move { play_match(&player, game, &pilot, &options).await });
            }
        }
        tokio::select! {
            Some(joined) = playing.join_next(), if !playing.is_empty() => {
                done += 1;
                each(joined.unwrap_or_else(|e| Err(anyhow!("the match's task ended: {e}"))));
            }
            () = tokio::time::sleep(settings.poll) => {}
        }
    }
    Ok(done)
}
