//! What a spectator is shown: the table as nobody in particular sees it.
//!
//! A type of its own rather than a [`PlayerView`] with fields left empty,
//! because private hands have no place in this type. Everything here
//! is what every seat at the table already sees alike: the public zones, the
//! life totals and counts, combat, and a library top only while it is
//! revealed to everyone. Continuously revealed hands are also public. No private
//! hand (only counts), no library, no face-down
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
    /// Hands revealed to everyone by a continuous rule, never private sharing.
    #[serde(default)]
    pub revealed_hands: Vec<crate::SharedHand>,
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
            announced_cast: None,
            announced_x: None,
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
            lost: Vec::new(),
            priority_held: false,
            policy_acts: Vec::new(),
            owed: None,
            monarch: self.monarch,
            day_night: self.day_night,
            seats: self.seats,
            hand: Vec::new(),
            shared_hands: self.revealed_hands,
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

    /// Every [`PrintRef`](baylee_core::ids::PrintRef) this public view names,
    /// in any zone, revealed hand or commander slot, without allocating a
    /// [`PlayerView`].
    pub fn prints(&self) -> impl Iterator<Item = baylee_core::ids::PrintRef> + '_ {
        let commanders = self
            .seats
            .iter()
            .flat_map(|s| &s.commanders)
            .filter_map(|c| c.card);
        let revealed = self
            .revealed_hands
            .iter()
            .flat_map(|h| &h.cards)
            .map(|o| o.card);
        let flat = self
            .battlefield
            .iter()
            .chain(&self.stack)
            .chain(&self.library_tops)
            .filter_map(|o| o.card);
        let nested = self
            .graveyards
            .iter()
            .chain(&self.exile)
            .chain(&self.command)
            .flatten()
            .filter_map(|o| o.card);
        commanders
            .chain(revealed)
            .chain(flat)
            .chain(nested)
            .map(|c| c.print)
    }
}
