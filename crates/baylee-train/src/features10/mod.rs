//! Encoder 10: Encoder 7's state and candidates, and on each target candidate
//! the side of the table it stands on against the side the effect wants.
//!
//! One signed value in candidate slot [`TARGET_SIDE_SLOT`], which no other
//! question writes: `+1` on the side the effect wants, `-1` on the other, `0`
//! where the question has no side to read or the candidate has none (a card in
//! a graveyard, "no target"). The net knows who controls a candidate and which
//! spell is asking; what it lacks is the cross of the two for cards it has seen
//! less, which this hands it. `TargetingContext::side` carries the reading, so
//! the value comes from the view alone, the same in conversion, in `eval4` and
//! at a live seat.
//!
//! Every other input is Encoder 7's. An export of another encoder reads zero
//! here, and an Encoder 10 export whose slot column is zero plays as Encoder 7.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TargetSide;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

use crate::features4::{Candidate4, EncodedCandidates4, EncodedState4};
use crate::features7::encode_candidates7_grouped;

/// Explicit observation contract. Existing defaults remain 7.
pub const ENCODER_VERSION10: u32 = 10;
/// The candidate slot of the side value; written by no other question.
pub const TARGET_SIDE_SLOT: usize = 12;

const fn stands(own: bool, side: TargetSide) -> f32 {
    match (own, side) {
        (true, TargetSide::Helpful) | (false, TargetSide::Harmful) => 1.0,
        _ => -1.0,
    }
}

/// A permanent or something on the stack, by its controller; anything else
/// is on neither side.
fn object_value(view: &PlayerView, side: TargetSide, id: ObjectId) -> f32 {
    view.battlefield
        .iter()
        .chain(&view.stack)
        .find(|o| o.id == id)
        .map_or(0.0, |o| stands(o.controller == view.seat, side))
}

fn player_value(view: &PlayerView, side: TargetSide, player: PlayerId) -> f32 {
    stands(player == view.seat, side)
}

/// Where `candidate` stands against `side`: `1`, `-1`, or `0` on neither side.
#[must_use]
pub fn candidate_side(candidate: &Candidate4, view: &PlayerView, side: TargetSide) -> f32 {
    let named = match &candidate.action {
        Some(PlayerAction::ChooseTargets { objects, players }) => {
            (objects.first().copied(), players.first().copied())
        }
        _ => (candidate.object_id, candidate.player_id),
    };
    match named {
        (Some(id), _) => object_value(view, side, id),
        (None, Some(player)) => player_value(view, side, player),
        (None, None) => 0.0,
    }
}

/// Encoder 7's candidates for `pending`, with the side value on target
/// candidates.
#[must_use]
pub fn encode_candidates10(
    state: &EncodedState4,
    view: &PlayerView,
    pending: &Pending,
) -> EncodedCandidates4 {
    let mut out = encode_candidates7_grouped(state, view, pending);
    if matches!(pending, Pending::ChooseTargets { .. })
        && let Some(side) = view.targeting.as_ref().and_then(|t| t.side)
    {
        for candidate in &mut out.candidates {
            candidate.feat[TARGET_SIDE_SLOT] = candidate_side(candidate, view, side);
        }
    }
    out
}

#[cfg(test)]
mod tests;
