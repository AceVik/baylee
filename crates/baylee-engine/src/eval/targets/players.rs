//! Player target choices and the players recorded on a stack object.

use crate::eval::players;
use crate::object::GameObject;
use crate::state::GameState;
use baylee_cards_dsl::{PlayerRel, TargetSpec};
use baylee_core::ids::PlayerId;

/// The players a spell or ability may target.
///
/// A player is untargetable only through an effect, never through a
/// characteristic — there is nothing on a player for a `Filter` to match —
/// so this is a list, not a filtered enumeration like the object half.
/// `Player(You)` here is an explicit target requirement; implicit self-effects
/// resolve their relation directly and never ask this targeting enumerator.
#[must_use]
pub fn target_player_options(state: &GameState, spec: &TargetSpec, you: PlayerId) -> Vec<PlayerId> {
    if !matches!(
        spec,
        TargetSpec::AnyTarget
            | TargetSpec::Player(PlayerRel::You)
            | TargetSpec::AnyPlayer
            | TargetSpec::AnyOpponent
            | TargetSpec::PlayerWhoAttackedThisTurn
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_)
    ) {
        return Vec::new();
    }
    let opponents_only = matches!(
        spec,
        TargetSpec::AnyOpponent | TargetSpec::OpponentOrObject(_)
    );
    let self_only = matches!(spec, TargetSpec::Player(PlayerRel::You));
    let attacked_only = matches!(spec, TargetSpec::PlayerWhoAttackedThisTurn);
    // Build the small seat set once instead of rescanning all effects for
    // each potential player target.
    let mut protected = baylee_core::ids::SeatSet::new();
    let mut everything = baylee_core::ids::SeatSet::new();
    let mut ignores_hexproof = false;
    for effect in state.effects.iter() {
        match effect.modifier {
            baylee_cards_dsl::Modifier::PlayerProtectionFromEverything { who } => {
                if let Some(players) = players(who, state, effect.controller) {
                    for player in players {
                        everything.insert(player);
                    }
                }
            }
            baylee_cards_dsl::Modifier::PlayerProtectionFromOpponents { who } => {
                if let Some(players) = players(who, state, effect.controller) {
                    for player in players {
                        if state.is_opponent(player, you) {
                            everything.insert(player);
                        }
                    }
                }
            }
            baylee_cards_dsl::Modifier::PlayerHexproof { who } => {
                if let Some(players) = players(who, state, effect.controller) {
                    for player in players {
                        protected.insert(player);
                    }
                }
            }
            baylee_cards_dsl::Modifier::IgnoreOpponentsHexproof if effect.controller == you => {
                ignores_hexproof = true;
            }
            _ => {}
        }
    }
    state
        .players
        .iter()
        .filter(|p| {
            if p.has_lost() || everything.contains(p.id) || (self_only && p.id != you) {
                return false;
            }
            // "Target opponent" is a choice over a smaller set, not a
            // different kind of choice (CR 115.1) — and the set is the
            // opponents, so a teammate is out of it as surely as you are.
            if opponents_only && !state.is_opponent(p.id, you) {
                return false;
            }
            if attacked_only
                && !state
                    .per_turn
                    .attacked
                    .iter()
                    .any(|&(_, _, who)| who == p.id)
            {
                return false;
            }
            // Player hexproof also stops only opponents (CR 702.11c).
            !state.is_opponent(p.id, you) || ignores_hexproof || !protected.contains(p.id)
        })
        .map(|p| p.id)
        .collect()
}

/// Every player the first instance of "target" of `obj` named, read off
/// the requirement the object carries: `target_players`, and
/// `chosen_player` where that requirement can name a player
/// ([`targeted_players`]).
///
/// For what is shown of a stack object. A "target player" spell or loyalty
/// ability is answered with `ChoosePlayer`, which the engine keeps in
/// `chosen_player` only, and the view and the log read `target_players`:
/// Ashiok, Dream Render's −1 stood on the stack aimed at nobody (a report
/// of 30.10.2026). The rules already read both, through `targeted_players`.
#[must_use]
pub fn first_target_players(obj: &GameObject) -> baylee_core::ids::SeatSet {
    let mut players = obj.target_players;
    if let Some(req) = obj.target_req {
        for player in targeted_players(obj, &req.spec).iter() {
            players.insert(player);
        }
    }
    players
}

/// The players a spell or ability on the stack chose for its first instance
/// of "target" under `spec`.
///
/// They ride in two fields. `target_players` is the set "any target" named,
/// and `chosen_player` is the one seat a player target named, but it is also
/// written by choices that are not targets at all. So the second is read
/// only for specs that can explicitly name a player, which is exactly the
/// set [`target_player_options`] answers for: folding it in for any other
/// spec would put a chosen player in front of an enumeration that never
/// offered them.
#[must_use]
pub fn targeted_players(obj: &GameObject, spec: &TargetSpec) -> baylee_core::ids::SeatSet {
    let mut players = baylee_core::ids::SeatSet::new();
    if matches!(
        spec,
        TargetSpec::AnyTarget
            | TargetSpec::Player(PlayerRel::You)
            | TargetSpec::AnyPlayer
            | TargetSpec::AnyOpponent
            | TargetSpec::PlayerWhoAttackedThisTurn
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_)
    ) {
        players = obj.target_players;
        if let Some(player) = obj.chosen_player {
            players.insert(player);
        }
    }
    players
}
