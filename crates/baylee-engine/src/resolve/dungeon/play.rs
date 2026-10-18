//! Room instructions that allow cards to be played during their resolution.
use super::{
    AwaitingOp, Cause, ChoicePrompt, GameEvent, GameState, Pending, Resolution, ZoneLocation,
    ZonePosition,
};
use crate::state::CardLookup as _;
use crate::zone::Zone;
use baylee_core::ids::DamageSourceRef;

/// Exact cards still available to this room's immediate play instruction.
#[derive(Clone, Debug, Hash)]
pub struct Play {
    /// Exact incarnations, so a returned card cannot inherit this offer.
    pub cards: Vec<DamageSourceRef>,
    /// Mad Wizard's Lair: one free spell from hand; otherwise ordinary plays from exile.
    pub free: bool,
}

pub(in crate::resolve) fn draw_three(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let Some((before, _)) = res.draw_watch.take() else {
        res.draw_watch = Some((state.per_turn.drawn.len(), state.draws_replaced));
        state.draw_cards(res.controller, 3);
        return None;
    };
    let cards: Vec<_> = state.per_turn.drawn[before..]
        .iter()
        .filter_map(|&(id, _)| state.object(id))
        .filter(|o| o.owner == res.controller && o.zone == Zone::Hand)
        .map(|o| DamageSourceRef {
            object: o.id,
            version: o.version,
        })
        .collect();
    state.journal.record(GameEvent::Revealed {
        player: res.controller,
        cards: cards.iter().map(|c| c.object).collect(),
    });
    continue_play(state, res, Play { cards, free: true })
}

pub(in crate::resolve) fn exile_two(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let mut cards = Vec::new();
    for _ in 0..2 {
        let Some(&card) = state
            .zones
            .list(ZoneLocation::Library(res.controller))
            .last()
        else {
            break;
        };
        let _ = state.move_object(
            card,
            ZoneLocation::Exile(res.controller),
            ZonePosition::Top,
            Cause::Effect,
        );
        if let Some(o) = state.object(card).filter(|o| o.zone == Zone::Exile) {
            cards.push(DamageSourceRef {
                object: o.id,
                version: o.version,
            });
        }
    }
    continue_play(state, res, Play { cards, free: false })
}

pub(crate) fn continue_play(
    state: &GameState,
    res: &mut Resolution,
    play: Play,
) -> Option<Pending> {
    let zone = if play.free { Zone::Hand } else { Zone::Exile };
    let options = play
        .cards
        .iter()
        .filter_map(|card| {
            state
                .object(card.object)
                .filter(|o| o.version == card.version && o.zone == zone)
        })
        .filter(|o| {
            !play.free
                || o.card
                    .and_then(|card| state.bases.card(card.index))
                    .is_some_and(|def| {
                        !def.faces[0]
                            .types
                            .contains(baylee_core::types::TypeSet::LAND)
                            || crate::casting::castable_back_faces(def, false)
                                .next()
                                .is_some()
                    })
        })
        .map(|o| o.id)
        .collect::<Vec<_>>();
    if options.is_empty() || state.has_left(res.controller) {
        return None;
    }
    let prompt = if play.free {
        ChoicePrompt::Generic
    } else {
        ChoicePrompt::PlayFromExile
    };
    res.awaiting = Some(AwaitingOp::DungeonPlay(play));
    Some(Pending::ChooseCards {
        player: res.controller,
        options,
        min: 0,
        max: 1,
        prompt,
        total: None,
    })
}
