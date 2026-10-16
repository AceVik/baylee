//! Tokens put onto the battlefield tapped and attacking
//! (`Effect::CreateTokenAttacking`, CR 508.4).
//!
//! The tokens are created tapped first; then, one token at a time, their
//! controller chooses what each attacks among the defending players and the
//! planeswalkers they control ([`crate::combat::defender_options`]). A single
//! option is taken without a question. With more, the player is asked first
//! (`Pending::ChoosePlayer`, skipped when only one player is left to attack),
//! then, if that player controls a planeswalker, whether the token attacks
//! one of those instead (`Pending::ChooseCards`, none chosen = the player).
//! Every token joins combat once the last is placed. Put onto the
//! battlefield attacking, they never "attacked" (CR 508.3a): no
//! `BecameAttacker` is journalled for them, and `per_turn.attacked` does not
//! name them.

use baylee_core::ids::{Defender, ObjectId, PlayerId};

use super::{AwaitingOp, Resolution};
use crate::choice::{ChoicePrompt, Pending};
use crate::state::GameState;

/// The tokens still to place, and where the placed ones attack.
#[derive(Clone, Debug, Hash)]
pub struct Tokens {
    /// Created tokens, in creation order.
    created: Vec<ObjectId>,
    /// What each placed token attacks, in the same order.
    placed: Vec<Defender>,
    /// The player chosen for the token being placed, before its
    /// planeswalker question.
    player: Option<PlayerId>,
}

/// Whether tokens `controller` creates now enter attacking: during a combat
/// of their own turn, once attackers can have been declared (CR 506.3b:
/// under any other player they only enter).
fn enters_attacking(state: &GameState, controller: PlayerId) -> bool {
    use crate::turn::Step;
    state.turn.phase == crate::turn::Phase::Combat
        && state.turn.active == controller
        && matches!(
            state.turn.step,
            Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::CombatDamageFirst
                | Step::CombatDamage
                | Step::CombatEnd
        )
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    tokens: Vec<ObjectId>,
) -> Option<Pending> {
    if tokens.is_empty() || !enters_attacking(state, res.controller) {
        return None;
    }
    ask(
        state,
        res,
        Tokens {
            created: tokens,
            placed: Vec::new(),
            player: None,
        },
    )
}

/// The answer to "which player": the token's planeswalker question follows
/// when that player controls one.
pub(super) fn resume_player(
    state: &mut GameState,
    res: &mut Resolution,
    mut tokens: Tokens,
    player: PlayerId,
) -> Option<Pending> {
    tokens.player = Some(player);
    ask(state, res, tokens)
}

/// The answer to "which planeswalker, if any".
pub(super) fn resume_walker(
    state: &mut GameState,
    res: &mut Resolution,
    mut tokens: Tokens,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let player = tokens.player.take();
    let defender = match chosen.first() {
        Some(&object) => crate::combat::defender_options(state, res.controller).into_iter()
            .find(|defender| matches!(defender, Defender::Planeswalker(id) | Defender::Battle(id) if *id == object))
            .unwrap_or(Defender::Battle(object)),
        None => Defender::Player(player.unwrap_or(res.controller)),
    };
    tokens.placed.push(defender);
    ask(state, res, tokens)
}

fn ask(state: &mut GameState, res: &mut Resolution, mut tokens: Tokens) -> Option<Pending> {
    let you = res.controller;
    while tokens.placed.len() < tokens.created.len() {
        let options = crate::combat::defender_options(state, you);
        let players: Vec<PlayerId> = options
            .iter()
            .filter_map(|d| match d {
                Defender::Player(p) => Some(*p),
                Defender::Planeswalker(_) | Defender::Battle(_) => None,
            })
            .collect();
        match options.as_slice() {
            // Nothing to attack: the rest enter only.
            [] => break,
            [only] => {
                tokens.placed.push(*only);
                continue;
            }
            _ => {}
        }
        let player = match (tokens.player, players.as_slice()) {
            (Some(player), _) => player,
            (None, [only]) => *only,
            (None, _) => {
                res.awaiting = Some(AwaitingOp::AttackingTokens(Box::new(tokens)));
                return Some(Pending::ChoosePlayer {
                    player: you,
                    options: players,
                });
            }
        };
        let walkers: Vec<ObjectId> = options
            .iter()
            .filter_map(|d| match d {
                Defender::Planeswalker(w) | Defender::Battle(w)
                    if crate::combat::blocking_player(state, *d) == Some(player) =>
                {
                    Some(*w)
                }
                _ => None,
            })
            .collect();
        if walkers.is_empty() {
            tokens.player = None;
            tokens.placed.push(Defender::Player(player));
            continue;
        }
        tokens.player = Some(player);
        res.awaiting = Some(AwaitingOp::AttackingTokens(Box::new(tokens)));
        return Some(Pending::ChooseCards {
            player: you,
            options: walkers,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::Generic,
            total: None,
        });
    }
    join_combat(state, &tokens);
    None
}

/// The placed tokens become attacking creatures, each attacking what was
/// chosen for it; a token that has left the battlefield meanwhile does not.
fn join_combat(state: &mut GameState, tokens: &Tokens) {
    let mut infos = Vec::new();
    for (&token, &defender) in tokens.created.iter().zip(&tokens.placed) {
        let Some(_) = state
            .object(token)
            .filter(|o| {
                o.zone == crate::zone::Zone::Battlefield
                    && o.controller == state.turn.active
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    && !o
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::BATTLE)
                    && crate::combat::defender_options(state, o.controller).contains(&defender)
            })
            .map(|o| o.version)
        else {
            continue;
        };
        infos.push(crate::combat::AttackerInfo {
            creature: token,
            defending: defender,
            blocked: false,
            band: None,
        });
        crate::combat::remember_attack(state, token, defender);
    }
    if !infos.is_empty() {
        state.combat.declare_attackers(infos);
        state.board_state_changed();
    }
}
