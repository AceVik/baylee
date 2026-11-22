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

/// Undercity's final room ("Throne of the Dead Three"): reveal the top ten
/// cards of your library, put a creature card from among them onto the
/// battlefield with three +1/+1 counters on it, grant it hexproof until your
/// next turn, then shuffle.
pub(in crate::resolve) fn throne_of_the_dead_three(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let top: Vec<baylee_core::ids::ObjectId> = state
        .zones
        .list(ZoneLocation::Library(you))
        .iter()
        .rev()
        .take(10)
        .copied()
        .collect();
    if !top.is_empty() {
        state.journal.record(GameEvent::Revealed {
            player: you,
            cards: top.clone(),
        });
    }
    let options: Vec<baylee_core::ids::ObjectId> = top
        .into_iter()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.is_card()
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .collect();
    if options.is_empty() {
        state.shuffle_library(you);
        return None;
    }
    res.awaiting = Some(AwaitingOp::UndercityThrone);
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::PutOntoBattlefield,
        total: None,
    })
}

pub(in crate::resolve) fn resume_throne(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[baylee_core::ids::ObjectId],
) {
    let you = res.controller;
    if let Some(&card) = chosen.first()
        && state
            .object(card)
            .is_some_and(|o| o.zone == Zone::Library && o.is_card())
    {
        let lib_version = state.object(card).map(|o| o.version);
        crate::resolve::search::put_found(
            state,
            you,
            card,
            baylee_cards_dsl::SearchDest::Battlefield,
        );
        let expected_version = lib_version.map(|v| v.wrapping_add(1));
        let entered = state.entries.contains(card)
            || state.object(card).is_some_and(|o| {
                o.zone == Zone::Battlefield && Some(o.version) == expected_version
            });
        if entered && let Some(version) = expected_version {
            crate::entry::put_entry_counters(state, card, baylee_cards_dsl::CounterKind::P1P1, 3);
            let timestamp = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: baylee_cards_dsl::Layer::Ability,
                timestamp,
                duration: baylee_cards_dsl::Duration::UntilYourNextTurn,
                filter: crate::effects::EffectFilter::ObjectIs(card, version),
                modifier: baylee_cards_dsl::Modifier::AddKeyword(
                    baylee_cards_dsl::KeywordSet::HEXPROOF,
                ),
            });
        }
    }
    state.shuffle_library(you);
}
