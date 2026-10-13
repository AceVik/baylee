//! `seen`: a digest of the public table, the same on every build.
//!
//! The engine's [`snapshot_hash`](baylee_engine::engine::Engine::snapshot_hash)
//! checks every bit of a replay, but only on the build that wrote the record:
//! any change to the engine's state, even one no player could see, moves it.
//! A replay on a later build is checked against `seen` instead: what the
//! table showed after each input, written in numbers of our own that mean
//! the same on every build (`docs/protocol.md` §"The game record").
//!
//! FNV-1a 64 over little-endian bytes, in this order, never an enum's
//! discriminant or its `Debug` text:
//!
//! 1. the turn (`u32`), the active seat (`u8`), the step in CR 500.1's order
//!    (`u8`, [`step_number`]);
//! 2. per seat in seat order: in the game (`u8`), life (`i32`), poison,
//!    hand, library and exile counts (`u32` each), the graveyard's cards in
//!    zone order (a `u32` count, then each card's `u32`);
//! 3. the battlefield as a sorted list of (card, controller `u8`, tapped
//!    `u8`, face down `u8`), a `u32` count first;
//! 4. the stack's size (`u32`).
//!
//! A card is its [`CardIndex`](baylee_core::ids::CardIndex), the append-only
//! ledger, so it means the same on every build; an object with no card (a
//! token) is [`NO_CARD`], a face-down permanent [`FACE_DOWN`] whatever it is.
//!
//! **Never change this encoding.** A golden test pins its value for a fixed
//! table; the day it has to say more, that is a new field (`seen2`) beside
//! this one, and records carrying `seen` still check against this function.

use baylee_engine::object::{GameObject, Status};
use baylee_engine::state::GameState;
use baylee_engine::turn::{Phase, Step, TurnInfo};
use baylee_engine::zone::ZoneLocation;

/// What an object with no card (a token) is written as.
pub const NO_CARD: u32 = u32::MAX;
/// What a face-down permanent is written as, whatever card it is.
pub const FACE_DOWN: u32 = u32::MAX - 1;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64, fed little-endian numbers.
struct Fnv(u64);

impl Fnv {
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }

    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }

    /// A count, saturated: no zone holds four billion objects.
    fn count(&mut self, n: usize) {
        self.u32(u32::try_from(n).unwrap_or(u32::MAX));
    }
}

/// The step as CR 500.1 orders a turn: untap 0, upkeep 1, draw 2, the
/// precombat main phase 3, beginning of combat 4, declare attackers 5,
/// declare blockers 6, combat damage 7 (first strike's too), end of combat 8,
/// the postcombat main phase 9, end 10, cleanup 11.
#[must_use]
pub const fn step_number(turn: &TurnInfo) -> u8 {
    match turn.step {
        Step::Untap => 0,
        Step::Upkeep => 1,
        Step::Draw => 2,
        Step::Main => match turn.phase {
            Phase::SecondMain | Phase::Ending => 9,
            Phase::Beginning | Phase::FirstMain | Phase::Combat => 3,
        },
        Step::CombatBegin => 4,
        Step::DeclareAttackers => 5,
        Step::DeclareBlockers => 6,
        Step::CombatDamageFirst | Step::CombatDamage => 7,
        Step::CombatEnd => 8,
        Step::End => 10,
        Step::Cleanup => 11,
    }
}

fn card_of(obj: &GameObject) -> u32 {
    obj.card.map_or(NO_CARD, |card| card.index.get())
}

/// The public table's digest (see the module docs).
#[must_use]
pub fn seen(state: &GameState) -> u64 {
    let mut h = Fnv(FNV_OFFSET);
    h.u32(state.turn.number);
    h.u8(state.turn.active.get());
    h.u8(step_number(&state.turn));

    for player in &state.players {
        let seat = player.id;
        h.u8(u8::from(!state.has_left(seat)));
        h.i32(player.life);
        h.u32(u32::from(player.poison));
        h.count(state.zones.list(ZoneLocation::Hand(seat)).len());
        h.count(state.zones.list(ZoneLocation::Library(seat)).len());
        h.count(state.zones.list(ZoneLocation::Exile(seat)).len());
        let graveyard = state.zones.list(ZoneLocation::Graveyard(seat));
        h.count(graveyard.len());
        for &id in graveyard {
            h.u32(state.object(id).map_or(NO_CARD, card_of));
        }
    }

    let mut battlefield: Vec<(u32, u8, u8, u8)> = state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter_map(|&id| state.object(id))
        .map(|obj| {
            let face_down = obj.status.contains(Status::FACE_DOWN);
            (
                if face_down { FACE_DOWN } else { card_of(obj) },
                obj.controller.get(),
                u8::from(obj.status.contains(Status::TAPPED)),
                u8::from(face_down),
            )
        })
        .collect();
    battlefield.sort_unstable();
    h.count(battlefield.len());
    for (card, controller, tapped, face_down) in battlefield {
        h.u32(card);
        h.u8(controller);
        h.u8(tapped);
        h.u8(face_down);
    }

    h.count(state.zones.list(ZoneLocation::Stack).len());
    h.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a 64's published test vectors: the hash is the hash.
    #[test]
    fn fnv_is_fnv_1a_64() {
        let mut empty = Fnv(FNV_OFFSET);
        empty.bytes(b"");
        assert_eq!(empty.0, 0xcbf2_9ce4_8422_2325);
        let mut a = Fnv(FNV_OFFSET);
        a.bytes(b"a");
        assert_eq!(a.0, 0xaf63_dc4c_8601_ec8c);
        let mut foobar = Fnv(FNV_OFFSET);
        foobar.bytes(b"foobar");
        assert_eq!(foobar.0, 0x8594_4171_f739_67e8);
    }
}
