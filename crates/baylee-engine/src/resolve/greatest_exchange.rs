//! Select each controller's greatest mana value, then exchange atomically.
use super::{AwaitingOp, ChoicePrompt, Pending, Resolution, control, eval, players_of, subjects};
use crate::state::GameState;
use baylee_cards_dsl::{Filter, PlayerRel};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

/// Suspended tie choices for one simultaneous control exchange.
#[derive(Clone, Debug, Hash)]
pub struct Selection {
    players: [PlayerId; 2],
    candidates: [Vec<DamageSourceRef>; 2],
    chosen: [Option<DamageSourceRef>; 2],
    next: usize,
}

impl Selection {
    pub(super) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(self)
    }
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    with: PlayerRel,
) -> Option<Pending> {
    let other = players_of(with, state, res.controller, res)
        .first()
        .copied()?;
    if other == res.controller || state.has_left(other) || state.has_left(res.controller) {
        return None;
    }
    // CR 101.4: simultaneous public choices use active-player turn order,
    // including a spell resolving outside its controller's turn.
    let mut players = [res.controller, other];
    let seats = state.players.len();
    let active = usize::from(state.turn.active.get());
    players.sort_by_key(|player| (usize::from(player.get()) + seats - active) % seats);
    let candidates = players.map(|player| greatest(state, res, player, filter));
    if candidates.iter().any(Vec::is_empty) {
        return None;
    }
    advance(
        state,
        res,
        Selection {
            players,
            candidates,
            chosen: [None; 2],
            next: 0,
        },
    )
}

fn greatest(
    state: &GameState,
    res: &Resolution,
    player: PlayerId,
    filter: &Filter,
) -> Vec<DamageSourceRef> {
    let mut maximum = None;
    let mut options = Vec::new();
    for id in state.battlefield_seen() {
        let Some(object) = state.object(id) else {
            continue;
        };
        if object.controller != player
            || !eval::matches_with_context(
                filter,
                state,
                object,
                res.controller,
                res.rule_context(),
            )
        {
            continue;
        }
        let value = object.characteristics().mana_value();
        if maximum.is_none_or(|old| value > old) {
            maximum = Some(value);
            options.clear();
        }
        if maximum == Some(value) {
            options.push(DamageSourceRef {
                object: id,
                version: object.version,
            });
        }
    }
    options
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let object = *chosen.first()?;
    selection.chosen[selection.next] = selection.candidates[selection.next]
        .iter()
        .find(|candidate| candidate.object == object)
        .copied();
    selection.next += 1;
    advance(state, res, selection)
}

fn advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
) -> Option<Pending> {
    while selection.next < 2 {
        let at = selection.next;
        if selection.candidates[at].len() == 1 {
            selection.chosen[at] = selection.candidates[at].first().copied();
            selection.next += 1;
        } else {
            let question = Pending::ChooseCards {
                player: selection.players[at],
                options: selection.candidates[at]
                    .iter()
                    .map(|candidate| candidate.object)
                    .collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            };
            res.awaiting = Some(AwaitingOp::ExchangeGreatest(Box::new(selection)));
            return Some(question);
        }
    }
    let [Some(first), Some(second)] = selection.chosen else {
        return None;
    };
    if selection
        .players
        .iter()
        .any(|player| state.has_left(*player))
    {
        return None;
    }
    if [first, second]
        .into_iter()
        .zip(selection.players)
        .all(|(object, player)| {
            subjects::on_battlefield(state, object)
                && state
                    .object(object.object)
                    .is_some_and(|object| object.controller == player)
        })
    {
        control::exchange(state, first.object, second.object);
    }
    None
}
