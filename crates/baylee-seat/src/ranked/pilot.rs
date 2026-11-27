//! What plays an AI account's seat: the one seam between the runner and the
//! mind. The runner signs in, offers, polls Central's work, takes each
//! match's admission, keeps its sessions and shuts down the same way
//! whatever flies the seat; a [`Pilot`] only plays one match's seat over
//! the table link it is handed, to its end.
//!
//! The pilots in this crate play through the bridge ([`fly_bridge`]): the
//! house at a fair level ([`HousePilot`]) and, through their profile, the
//! language models. A pilot outside it (the trained net, which plays its own
//! core over the same link) implements [`Pilot`] the same way. One that
//! cannot go on, because a resumed socket lost observations its memory
//! needs and a snapshot cannot rebuild, says so ([`Piloted::HandOver`]):
//! the runner then plays the rest of the game with its declared house
//! fallback on the same link, which declares itself to the table, so the
//! game's record names the house from there on.

use super::{Frozen, RankedLink};
use crate::bridge::{self, PlayOptions, Played};
use crate::link::Table;
use crate::mind::Mind;
use crate::seat::{BridgeConfig, Labelled, SeatCore};
use crate::transcript::Transcript;
use anyhow::anyhow;
use baylee_account::ranked::ai::AiKind;
use baylee_ai::AIProfile;
use baylee_protocol::v1::SeatMind;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// One match's seat, handed to its pilot.
pub struct Seat<'a> {
    /// The way to the table: Central's relay (connect, resume, renewal).
    pub link: &'a mut RankedLink,
    /// The deck the match was made with.
    pub deck: &'a Frozen,
    /// Where the match's questions and answers are kept.
    pub transcript: &'a mut Transcript,
    /// The bridge's pacing (the least think time a person can follow).
    pub play: &'a PlayOptions,
}

/// How a pilot's flight ended.
#[derive(Debug)]
pub enum Piloted {
    /// It played the seat to the game's end (or to the room's close).
    Played(Box<Played>),
    /// It cannot go on, and says why: the house fallback takes the seat.
    HandOver(String),
}

/// A pilot's flight: one match's seat played out.
pub type Flight<'a> = Pin<Box<dyn Future<Output = anyhow::Result<Piloted>> + Send + 'a>>;

/// What plays an AI account's seats.
pub trait Pilot: Send + Sync {
    /// What the seat says plays it, for the game's record
    /// (`docs/protocol.md` §"Who answers a seat, as it says").
    fn declared(&self) -> SeatMind;

    /// The kind of mind Central's administrator set for the account: the
    /// runner plays an account only with a pilot of its kind.
    fn kind(&self) -> AiKind;

    /// The runner's key for this mind, as Central's administrator set it
    /// for the account (`house:expert`): the runner plays an account only
    /// with the pilot its settings name.
    fn profile(&self) -> String;

    /// Whether it can take a new game now; `Err` says why not, and the
    /// account makes no offer meanwhile (a language model with no access
    /// path left).
    ///
    /// # Errors
    /// Why it cannot.
    fn ready(&self) -> Result<(), String> {
        Ok(())
    }

    /// Plays `seat` to the game's end, or hands it over.
    fn fly<'a>(&'a self, seat: Seat<'a>) -> Flight<'a>;
}

/// Plays a seat through any `link` with the bridge: `mind` answers, `house`
/// where it cannot, held to the account's label (a ranked host names its
/// chairs itself), declared as `declared`.
///
/// # Errors
/// When the seat leaves the table or cannot reach it again.
#[allow(clippy::too_many_arguments)]
pub async fn fly_bridge<L: Table>(
    link: &mut L,
    deck: &Frozen,
    transcript: &mut Transcript,
    play: &PlayOptions,
    mind: Arc<dyn Mind>,
    declared: SeatMind,
    house: AIProfile,
    think: Duration,
) -> anyhow::Result<Played> {
    let config = BridgeConfig {
        think,
        house,
        labelled: Labelled::Account,
        ..BridgeConfig::default()
    };
    let core =
        SeatCore::new(config, deck.bridge.list.clone(), mind.disclosure()).with_mind(declared);
    bridge::play(link, core, mind, transcript, play).await
}

/// The house at a fair level: what Central's `house:<level>` names.
#[derive(Clone, Debug)]
pub struct HousePilot {
    level: String,
    profile: AIProfile,
    /// The longest one answer may take.
    pub think: Duration,
}

impl HousePilot {
    /// The house at `level` (`expert`); never `seer`, which reads hidden
    /// cards a seat is never sent ([`crate::house::level`]).
    ///
    /// # Errors
    /// A level the house has not, or one that peeks.
    pub fn new(level: &str) -> anyhow::Result<Self> {
        let profile = crate::house::level(level).map_err(|why| anyhow!(why))?;
        Ok(Self {
            level: level.to_owned(),
            profile,
            think: Duration::from_secs(30),
        })
    }

    /// Its level.
    #[must_use]
    pub fn level(&self) -> &str {
        &self.level
    }

    /// Plays through any `link` (a test's, which cuts the socket).
    ///
    /// # Errors
    /// As [`fly_bridge`].
    pub async fn fly_through<L: Table>(
        &self,
        link: &mut L,
        deck: &Frozen,
        transcript: &mut Transcript,
        play: &PlayOptions,
    ) -> anyhow::Result<Played> {
        let mind = Arc::new(crate::HouseMind::new(self.profile)) as Arc<dyn Mind>;
        fly_bridge(
            link,
            deck,
            transcript,
            play,
            mind,
            self.declared(),
            self.profile,
            self.think,
        )
        .await
    }
}

impl Pilot for HousePilot {
    fn declared(&self) -> SeatMind {
        crate::declare::house(&self.level)
    }

    fn kind(&self) -> AiKind {
        AiKind::House
    }

    fn profile(&self) -> String {
        format!("house:{}", self.level)
    }

    fn fly<'a>(&'a self, seat: Seat<'a>) -> Flight<'a> {
        Box::pin(async move {
            self.fly_through(seat.link, seat.deck, seat.transcript, seat.play)
                .await
                .map(|played| Piloted::Played(Box::new(played)))
        })
    }
}
