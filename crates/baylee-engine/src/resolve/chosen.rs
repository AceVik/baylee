//! One player at a time, picking one of their own permanents.
//!
//! Two effects share a shape rather than a meaning: the effect names a
//! filter and a set of players, and every one of those players is asked, in
//! turn, to name one of their own permanents matching it. Only what happens
//! to the pick differs — [`Effect::SacrificeFilter`] sends it to the
//! graveyard and [`Effect::DestroyChosenForPlayers`] destroys it.
//!
//! Two questions are the same in all of them and live here. What a given
//! player may pick ([`options`]), and who is asked next ([`next_asked`]) —
//! which is not simply "the first player left", because a player with no
//! legal pick is skipped rather than shown an empty list.
//!
//! What deliberately does **not** live here is `min`. It reads as a detail
//! and is the sentence: "sacrifices a creature" is `min: 1` and the player
//! must give something up, while "destroy **up to one**" is `min: 0` and
//! they may decline. A helper that averaged the two would silently make one
//! of those cards wrong, so each call site states its own.
//!
//! [`Effect::SacrificeFilter`]: baylee_cards_dsl::Effect::SacrificeFilter
//! [`Effect::DestroyChosenForPlayers`]: baylee_cards_dsl::Effect::DestroyChosenForPlayers

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

use crate::eval;
use crate::state::GameState;
use crate::zone::ZoneLocation;

/// The permanents `player` controls that `filter` matches.
///
/// `you` and `source` are the effect's own controller and source, which is
/// what a filter reads "you" and "this" as — not the player being asked.
pub(super) fn options(
    state: &GameState,
    player: PlayerId,
    filter: &'static Filter,
    you: PlayerId,
    source: ObjectId,
) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                o.controller == player && eval::matches(filter, state, o, you, source)
            })
        })
        .copied()
        .collect()
}

/// Takes players off the front of `remaining` until one has a legal pick,
/// and hands back that player with their options.
///
/// `None` means nobody left can pick, which ends the chain. The skipping is
/// the point: a player with nothing matching is not asked an unanswerable
/// question, and `Pending::ChooseCards` with an empty list and `min: 1`
/// would be exactly that.
pub(super) fn next_asked(
    state: &GameState,
    remaining: &mut Vec<PlayerId>,
    filter: &'static Filter,
    you: PlayerId,
    source: ObjectId,
) -> Option<(PlayerId, Vec<ObjectId>)> {
    while !remaining.is_empty() {
        let player = remaining.remove(0);
        let options = options(state, player, filter, you, source);
        if !options.is_empty() {
            return Some((player, options));
        }
    }
    None
}
