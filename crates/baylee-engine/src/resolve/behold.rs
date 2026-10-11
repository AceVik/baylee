//! The optional reveal-or-choose action of CR 701.4.
use super::{AwaitingOp, ChoicePrompt, Pending, Resolution, eval};
use crate::{state::GameState, zone::ZoneLocation};
use baylee_cards_dsl::{Effect, Filter};

pub(super) fn start(
    state: &GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    then: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let options: Vec<_> = state
        .zones
        .list(ZoneLocation::Hand(you))
        .iter()
        .copied()
        .chain(state.battlefield_seen().filter(|id| {
            state
                .object(*id)
                .is_some_and(|object| object.controller == you)
        }))
        .filter(|id| {
            state.object(*id).is_some_and(|object| {
                eval::matches_with_context(filter, state, object, you, res.rule_context())
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::Behold { then });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Behold,
        total: None,
    })
}
