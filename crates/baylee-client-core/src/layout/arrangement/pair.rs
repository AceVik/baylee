//! The pair (DESIGN-v8 §1 rows 3, 6 and 8): my side and the seat of
//! interest's side seated as a duel. Spotlight parks everybody else; the
//! Turntable stands them on the flanks; the Focus ring parks them behind
//! peeks.

use super::super::{Seat, SeatSlot, TableLayout, sides_of};
use baylee_core::ids::PlayerId;
use glam::Vec2;

/// How far past the pair's own reach a parked seat stands at least: off
/// the slab, so its cards glide in from outside the pair's frame and are
/// never drawn on the felt while they wait (they are hidden there besides).
pub const PARK_REACH: f32 = 20.0;

/// The side the pair seats across from mine, as indices into `seats`: the
/// side of `interest`, when that is a seat on another side than mine; else
/// the side across the ring from mine (the middle of the others in turn
/// order), which is what the table shows at home.
#[must_use]
pub fn across_side(seats: &[Seat], interest: Option<PlayerId>) -> Vec<usize> {
    let sides = sides_of(seats);
    let chosen = interest.and_then(|player| {
        sides
            .iter()
            .position(|side| side.iter().any(|&i| seats[i].player == player))
    });
    let index = match chosen {
        Some(i) if i != 0 => i,
        _ => sides.len() / 2,
    };
    sides.get(index).cloned().unwrap_or_default()
}

/// My side and `across` seated as a duel (`seated` of the two sides, the
/// roomy branch where both are single seats), returned as one slot per seat
/// of the whole roster — the pair's from that duel, every other seat
/// `parked` far out along its ring bearing. Every slot keeps the ring's
/// `angle`, the bearing the dial's jewel stands at for a parked seat.
#[must_use]
pub fn pair(seats: &[Seat], aspect: f32, across: &[usize]) -> TableLayout {
    let sides = sides_of(seats);
    let mine = sides.first().cloned().unwrap_or_default();
    let members: Vec<usize> = mine.iter().chain(across).copied().collect();
    let pair_seats: Vec<Seat> = members.iter().map(|&i| seats[i]).collect();
    let duel = TableLayout::seated(&pair_seats, aspect, None);
    let ring = TableLayout::seated(seats, aspect, None);
    let reach = duel
        .extent()
        .map_or(0.0, |(lo, hi)| lo.abs().max(hi.abs()).max_element());
    let mut slots = Vec::with_capacity(seats.len());
    for (i, seat) in seats.iter().enumerate() {
        let on_ring = ring.slots[i];
        let slot = members.iter().position(|&m| m == i).map_or_else(
            || {
                // Where the ring would seat it, or further out along the
                // same bearing where that is inside the pair's reach: a
                // switch to the ring then finds its cards already home.
                let bearing = on_ring.center.normalize_or(Vec2::Y);
                let out = on_ring.center.length().max(reach + PARK_REACH);
                SeatSlot {
                    center: bearing * out,
                    parked: true,
                    ..on_ring
                }
            },
            |k| SeatSlot {
                player: seat.player,
                ring_index: i,
                angle: on_ring.angle,
                is_local: i == 0,
                ..duel.slots[k]
            },
        );
        slots.push(slot);
    }
    TableLayout {
        slots,
        radius: duel.radius,
    }
}

/// Spotlight (DESIGN-v8 §1 row 6, *lite*): the pair, everybody else parked
/// and read from their strip chips.
#[must_use]
pub fn spotlight(seats: &[Seat], aspect: f32, interest: Option<PlayerId>) -> TableLayout {
    pair(seats, aspect, &across_side(seats, interest))
}
