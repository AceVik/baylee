//! What a spectator is shown: the table as nobody in particular sees it.
//!
//! A type of its own rather than a [`PlayerView`] with fields left empty,
//! because a field that does not exist cannot carry a hand. Everything here
//! is what every seat at the table already sees alike: the public zones, the
//! life totals and counts, combat, and a library top only while it is
//! revealed to everyone. No hand (only counts), no library, no face-down
//! card's identity, no question, no clock, nothing a choice put in front of
//! one seat. `docs/protocol.md` §"Spectators".

use crate::combat::CombatView;
use crate::objects::PublicObject;
use crate::player_view::PlayerView;
use crate::seats::SeatView;
use crate::turn::{DayNight, Phase, Step};
use baylee_core::ids::{PlayerId, SeatSet};
use serde::{Deserialize, Serialize};

/// The public table, for a spectator.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SpectatorView {
    /// Monotonic per game, as a seat's view.
    pub seq: u64,
    /// Turn number.
    pub turn: u32,
    /// Phase.
    pub phase: Phase,
    /// Step.
    pub step: Step,
    /// The active player.
    pub active: PlayerId,
    /// The seat the table waits for, as every seat but the asked one sees it.
    pub awaiting: Option<PlayerId>,
    /// Seats still deciding their opening mulligan.
    pub deciding: SeatSet,
    /// The monarch, if any.
    pub monarch: Option<PlayerId>,
    /// Day or night, once it has become either.
    pub day_night: Option<DayNight>,
    /// Every seat's public numbers: life, counts, floating mana.
    pub seats: Vec<SeatView>,
    /// The battlefield; a face-down permanent is a blank to a spectator.
    pub battlefield: Vec<PublicObject>,
    /// The stack.
    pub stack: Vec<PublicObject>,
    /// Graveyards, per seat.
    pub graveyards: Vec<Vec<PublicObject>>,
    /// Exile, per seat; a face-down card stays a blank.
    pub exile: Vec<Vec<PublicObject>>,
    /// Command zones, per seat.
    pub command: Vec<Vec<PublicObject>>,
    /// The combat in progress.
    pub combat: CombatView,
    /// Library tops revealed to the whole table.
    pub library_tops: Vec<PublicObject>,
}

impl SpectatorView {
    /// The table as a seat-shaped view a renderer can draw, seen from
    /// `perspective`: an empty hand, no question, no clock.
    ///
    /// The private fields are empty because this type has nothing to put in
    /// them; a client that draws a seat's view draws a spectator's with no
    /// second code path.
    #[must_use]
    pub fn into_player_view(self, perspective: PlayerId) -> PlayerView {
        PlayerView {
            decision_player: None,
            controlled_hands: Vec::new(),
            damage_sources: Vec::new(),
            target_objects: Vec::new(),
            targeting: None,
            casting: None,
            seq: self.seq,
            seat: perspective,
            turn: self.turn,
            phase: self.phase,
            step: self.step,
            active: self.active,
            awaiting: self.awaiting,
            deciding: self.deciding,
            decision_remaining_ms: None,
            clocks: Vec::new(),
            priority_held: false,
            policy_acts: Vec::new(),
            owed: None,
            monarch: self.monarch,
            day_night: self.day_night,
            seats: self.seats,
            hand: Vec::new(),
            shared_hands: Vec::new(),
            hand_shared_with: SeatSet::new(),
            hand_requests: SeatSet::new(),
            hand_requested: SeatSet::new(),
            battlefield: self.battlefield,
            stack: self.stack,
            graveyards: self.graveyards,
            exile: self.exile,
            command: self.command,
            combat: self.combat,
            looking_at: Vec::new(),
            library_tops: self.library_tops,
            sorcery_lock: None,
            sorceries_have_flash: false,
        }
    }
}
