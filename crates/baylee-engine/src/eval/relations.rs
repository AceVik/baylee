//! Which players a relation names, and who controls or defends against a source.

use super::last_defender;
use crate::state::GameState;
use baylee_cards_dsl::PlayerRel;
use baylee_core::ids::{ObjectId, PlayerId};

/// Who controls the permanent `source` is attached to: "enchanted land's
/// controller", which need not be the Aura's own (CR 303.4e). `None` when the
/// source is gone or attached to nothing, or its host has left the game's
/// players.
#[must_use]
pub fn controller_of_attached(state: &GameState, source: ObjectId) -> Option<PlayerId> {
    let host = state.object(source)?.attached_to?;
    let seat = state.object(host)?.controller;
    (!state.has_left(seat)).then_some(seat)
}

/// Resolves a relative player reference to concrete players — or `None` for
/// relations the game state alone cannot answer.
///
/// `Chosen` is the player a spell or ability targeted and `ControllerOfTarget`
/// is read off its first object target, so both need the *resolution's* own
/// context and only [`crate::resolve::players_of`] has it. The `None` is the
/// whole point of the signature: this used to answer `vec![]` there, which
/// reads exactly like "no seats matched" and is what shipped Abraded Bluffs
/// as a land that deals no damage, Twining Twins' ward as a keyword that never
/// asks for its tax, Path to Exile without its ramp and Bojuka Bog as a swamp.
/// A caller that has a `Resolution` must not be able to swallow that silently.
#[must_use]
pub fn players(rel: PlayerRel, state: &GameState, you: PlayerId) -> Option<Vec<PlayerId>> {
    Some(match rel {
        PlayerRel::You => vec![you],
        PlayerRel::Opponent | PlayerRel::EachOpponent => state
            .players
            .iter()
            .filter(|p| state.is_opponent(p.id, you) && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachPlayer => state
            .players
            .iter()
            .filter(|p| !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachOtherPlayer => state
            .players
            .iter()
            .filter(|p| p.id != you && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::ActivePlayer => state
            .players
            .iter()
            .filter(|p| p.id == state.turn.active && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::MostLife => most_life(state).into_iter().collect(),
        PlayerRel::MostLifeOrTied => {
            let best = state
                .players
                .iter()
                .filter(|p| !p.has_lost())
                .map(|p| p.life)
                .max();
            state
                .players
                .iter()
                .filter(|p| !p.has_lost() && Some(p.life) == best)
                .map(|p| p.id)
                .collect()
        }
        PlayerRel::LeastLifeOrTied => {
            let least = state
                .players
                .iter()
                .filter(|p| !p.has_lost())
                .map(|p| p.life)
                .min();
            state
                .players
                .iter()
                .filter(|p| !p.has_lost() && Some(p.life) == least)
                .map(|p| p.id)
                .collect()
        }
        PlayerRel::ControllerOfTarget
        | PlayerRel::TargetPlayerOrController
        | PlayerRel::SecondTargetPlayerOrController
        | PlayerRel::ControllerOfEvent
        | PlayerRel::Recipient
        | PlayerRel::DefendingPlayer
        | PlayerRel::OwnerOfEvent
        | PlayerRel::Chosen
        | PlayerRel::DamagedPlayer
        | PlayerRel::EventPlayer
        | PlayerRel::OwnerOfSource
        | PlayerRel::ControllerOfAttached => {
            return None;
        }
    })
}

/// The one player still in the game with more life than each other one
/// ("the player with the most life", Ghazbán Ogre), or `None` on a tie.
#[must_use]
pub fn most_life(state: &GameState) -> Option<PlayerId> {
    let mut alive = state.players.iter().filter(|p| !p.has_lost());
    let first = alive.next()?;
    let (mut best, mut tied) = (first, false);
    for p in alive {
        if p.life > best.life {
            best = p;
            tied = false;
        } else if p.life == best.life {
            tied = true;
        }
    }
    (!tied).then_some(best.id)
}

/// "Defending player" as an ability of `source` means it (CR 508.5): the
/// player `source` is attacking, or the controller of the planeswalker or
/// protector of the battle it is attacking, and once it has been removed
/// from combat, the one it was attacking before
/// ([`crate::combat::CombatState::last_defender_of`]). A source that did
/// not attack this combat names the one opponent of a two-player game
/// during combat (CR 506.2), and nobody at a larger table, where CR 508.5a
/// makes it one specific player that no attack names.
#[must_use]
pub fn defending_player_of(state: &GameState, source: ObjectId, you: PlayerId) -> Option<PlayerId> {
    // Attacking, or attacking earlier this combat and removed since (CR
    // 508.5: "the player that creature was attacking before it was removed
    // from combat").
    if last_defender(state, source).is_some() {
        return crate::combat::historical_defending_player(state, source)
            .filter(|p| !state.has_left(*p));
    }
    if state.turn.phase != crate::turn::Phase::Combat {
        return None;
    }
    let mut opponents = state
        .players
        .iter()
        .filter(|p| state.is_opponent(p.id, you) && !p.has_lost());
    match (opponents.next(), opponents.next()) {
        (Some(only), None) => Some(only.id),
        _ => None,
    }
}
