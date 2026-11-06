//! Keep one controlled permanent per listed kind, then sacrifice the rest.
//!
//! Choices are public and happen in APNAP order. Nothing moves between those
//! choices and the single simultaneous sacrifice (Cataclysm).

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

use super::{AwaitingOp, Resolution, players_of};
use crate::choice::{ChoicePrompt, Pending};
use crate::event::GameEvent;
use crate::state::GameState;
use crate::zone::Zone;

#[derive(Clone, Debug, Hash)]
struct Group {
    player: PlayerId,
    cards: Vec<ObjectId>,
    kept: Vec<ObjectId>,
}

/// A suspended set of per-kind choices. Every selected object remains in place.
#[derive(Clone, Debug, Hash)]
pub struct Selection {
    filters: &'static [&'static Filter],
    groups: Vec<Group>,
    current: usize,
    category: usize,
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filters: &'static [&'static Filter],
) -> Option<Pending> {
    if filters.is_empty() {
        return None;
    }
    let seats = state.players.len();
    let active = usize::from(state.turn.active.get());
    let mut players = players_of(
        baylee_cards_dsl::PlayerRel::EachPlayer,
        state,
        res.controller,
        res,
    );
    players.sort_by_key(|player| (usize::from(player.get()) + seats - active) % seats);
    let groups = players
        .into_iter()
        .map(|player| Group {
            player,
            cards: crate::eval::controlled_matching_with_context(
                state,
                player,
                &Filter::Any,
                res.controller,
                res.rule_context(),
            ),
            kept: Vec::new(),
        })
        .collect();
    advance(
        state,
        res,
        Selection {
            filters,
            groups,
            current: 0,
            category: 0,
        },
    )
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let group = selection.groups.get(selection.current)?;
    if state.has_left(group.player) {
        selection.current += 1;
        selection.category = 0;
        return advance(state, res, selection);
    }

    // An empty answer is how the departure path declines the old question.
    // It must not choose an arbitrary permanent on the player's behalf.
    if chosen.len() == 1
        && options(state, res, &selection).contains(&chosen[0])
        && let Some(group) = selection.groups.get_mut(selection.current)
        && !group.kept.contains(&chosen[0])
    {
        group.kept.push(chosen[0]);
    }
    selection.category += 1;
    advance(state, res, selection)
}

fn options(state: &GameState, res: &Resolution, selection: &Selection) -> Vec<ObjectId> {
    let Some(group) = selection.groups.get(selection.current) else {
        return Vec::new();
    };
    let Some(filter) = selection.filters.get(selection.category) else {
        return Vec::new();
    };
    group
        .cards
        .iter()
        .copied()
        .filter(|&card| {
            state.object(card).is_some_and(|object| {
                object.zone == Zone::Battlefield
                    && object.controller == group.player
                    && !object.status.contains(crate::object::Status::PHASED_OUT)
                    && crate::eval::matches_with_context(
                        filter,
                        state,
                        object,
                        res.controller,
                        res.rule_context(),
                    )
            })
        })
        .collect()
}

fn advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
) -> Option<Pending> {
    while selection.current < selection.groups.len() {
        let player = selection.groups[selection.current].player;
        if state.has_left(player) {
            selection.current += 1;
            selection.category = 0;
            continue;
        }
        if selection.category < selection.filters.len() {
            let options = options(state, res, &selection);
            match options.len() {
                0 => selection.category += 1,
                1 => {
                    let kept = options[0];
                    let group = &mut selection.groups[selection.current];
                    if !group.kept.contains(&kept) {
                        group.kept.push(kept);
                    }
                    selection.category += 1;
                }
                _ => {
                    let pending = Pending::ChooseCards {
                        player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::KeepPermanents,
                        total: None,
                    };
                    res.awaiting = Some(AwaitingOp::KeepOneOfEach(Box::new(selection)));
                    return Some(pending);
                }
            }
            continue;
        }

        let group = &selection.groups[selection.current];
        if !group.kept.is_empty() {
            state.journal.record(GameEvent::CardsKept {
                player,
                cards: group.kept.clone(),
            });
        }
        selection.current += 1;
        selection.category = 0;
    }

    // Departures can end continuous control changes. Sacrifice only objects
    // still controlled by the player whose original group contains them.
    let current: &GameState = state;
    let victims: Vec<_> = selection
        .groups
        .into_iter()
        .filter(|group| !current.has_left(group.player))
        .flat_map(|group| {
            group.cards.into_iter().filter(move |card| {
                !group.kept.contains(card)
                    && current.object(*card).is_some_and(|object| {
                        object.zone == Zone::Battlefield
                            && object.controller == group.player
                            && !object.status.contains(crate::object::Status::PHASED_OUT)
                    })
            })
        })
        .collect();
    super::sacrifice_together(state, &victims);
    None
}
