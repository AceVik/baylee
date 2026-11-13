//! One acquisition's linked exile, private inspections, and immediate play offers.
use super::{
    AwaitingOp, ChoicePrompt, GameState, ObjectId, Pending, Resolution, ZoneLocation, ZonePosition,
    ask_commander_replace, bottom_in_random_order, source_version,
};
use crate::event::Cause;
use crate::object::Rider;
use crate::zone::Zone;
use baylee_cards_dsl::ZoneRef;
use baylee_core::ids::{DamageSourceRef, PlayerId, SeatSet};

fn link(state: &GameState, res: &Resolution) -> Option<Rider> {
    let source = state
        .recorded_ability_source(res.on_stack)
        .or_else(|| super::subjects::source(state, res))?;
    Some(Rider::LinkedExile {
        source,
        acquisition: super::subjects::acquisition(state, res),
    })
}

fn linked(state: &GameState, res: &Resolution) -> Vec<DamageSourceRef> {
    let Some(link) = link(state, res) else {
        return Vec::new();
    };
    (0..state.players.len())
        .flat_map(|i| {
            state.zones.list(ZoneLocation::Exile(PlayerId::new(
                u8::try_from(i).expect("seat fits"),
            )))
        })
        .filter_map(|&id| state.object(id))
        .filter(|o| o.riders.contains(&link))
        .map(|o| DamageSourceRef {
            object: o.id,
            version: o.version,
        })
        .collect()
}

pub(super) fn hideaway(state: &GameState, res: &mut Resolution, count: u8) -> Option<Pending> {
    let looked: Vec<_> = state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .iter()
        .rev()
        .take(usize::from(count))
        .copied()
        .collect();
    if looked.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::Hideaway {
        looked: looked.clone(),
    });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: looked,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    })
}

pub(super) fn choose(
    state: &mut GameState,
    res: &Resolution,
    looked: Vec<ObjectId>,
    chosen: &[ObjectId],
) {
    let library = ZoneLocation::Library(res.controller);
    let mut rest: Vec<_> = looked
        .into_iter()
        .filter(|&id| state.zones.contains(id, library))
        .collect();
    if let Some(&card) = chosen.first().filter(|id| rest.contains(id)) {
        exile(state, res, card, true);
        rest.retain(|&id| id != card);
    }
    bottom_in_random_order(state, res.controller, rest);
}

/// Privately inspect a bounded top section for one face-down exile choice.
pub(super) fn look_at_top(
    state: &mut GameState,
    res: &mut Resolution,
    count: u8,
    cast_types: baylee_core::types::TypeSet,
) -> Option<Pending> {
    let looked: Vec<_> = state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .iter()
        .rev()
        .take(usize::from(count))
        .copied()
        .collect();
    if looked.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::LookedCardToExile {
        looked: looked.clone(),
        cast_types,
    });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: looked,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    })
}

/// Exile the chosen card privately, then let its controller order the rest.
pub(super) fn finish_look_at_top(
    state: &mut GameState,
    res: &mut Resolution,
    looked: Vec<ObjectId>,
    cast_types: baylee_core::types::TypeSet,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let library = ZoneLocation::Library(res.controller);
    let &card = chosen
        .first()
        .filter(|card| looked.contains(card) && state.zones.contains(**card, library))?;
    let owner = state
        .object(card)
        .map_or(res.controller, |object| object.owner);
    if let Some(pending) = ask_commander_replace(state, res, &[(card, ZoneLocation::Exile(owner))])
    {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("replacement question retains its owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::LookedCardToExile { looked, cast_types }),
            chosen: vec![card],
            asked,
            remaining,
        });
        return Some(pending);
    }

    if let Some(Rider::LinkedExile {
        source,
        acquisition,
    }) = link(state, res)
        && state
            .move_object(
                card,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            )
            .is_ok()
        && state
            .object(card)
            .is_some_and(|object| object.zone == Zone::Exile)
    {
        let seen = std::iter::once(res.controller).collect();
        state.hide_exiled_card(card, source, acquisition, seen, false);
        let version = state.object(card).expect("the exiled card").version;
        state.play_permissions.push(crate::state::PlayPermission {
            player: res.controller,
            card,
            version,
            free: false,
            mana_any_type: false,
            cast_only: true,
            spell_types: Some(cast_types),
            window: crate::state::PlayLifetime::WhileExiled,
        });
    }

    let rest: Vec<_> = looked.into_iter().filter(|id| *id != card).collect();
    if rest.len() > 1 {
        res.awaiting = Some(AwaitingOp::DigBottom);
        let count = u32::try_from(rest.len()).unwrap_or(u32::MAX);
        return Some(Pending::Arrange {
            player: res.controller,
            cards: rest,
            piles: vec![crate::choice::ArrangePile::all_of(
                crate::choice::ArrangePlace::LibraryBottom,
                count,
            )],
            prompt: crate::choice::ArrangePrompt::Order,
        });
    }
    for card in rest {
        let _ = state.move_object(card, library, ZonePosition::Bottom, Cause::Effect);
    }
    None
}

pub(super) fn exile_top(state: &mut GameState, res: &Resolution) {
    if let Some(&card) = state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .last()
    {
        exile(state, res, card, false);
    }
}

fn exile(state: &mut GameState, res: &Resolution, card: ObjectId, hideaway: bool) {
    let Some(Rider::LinkedExile {
        source,
        acquisition,
    }) = link(state, res)
    else {
        return;
    };
    let Some(owner) = state.object(card).map(|o| o.owner) else {
        return;
    };
    if state
        .move_object(
            card,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .is_err()
    {
        return;
    }
    let mut seen = SeatSet::new();
    if hideaway {
        seen.insert(res.controller);
    }
    state.hide_exiled_card(card, source, acquisition, seen, hideaway);
}

pub(super) fn offer_play(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let cards = linked(state, res);
    continue_play(state, res, cards, false)
}

pub(super) fn offer_copy(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let cards = linked(state, res);
    continue_play(state, res, cards, true)
}

pub(crate) fn continue_play(
    state: &GameState,
    res: &mut Resolution,
    cards: Vec<DamageSourceRef>,
    copy: bool,
) -> Option<Pending> {
    let options: Vec<_> = cards
        .iter()
        .filter_map(|r| {
            state
                .object(r.object)
                .filter(|o| o.zone == Zone::Exile && o.version == r.version)
                .map(|o| o.id)
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::LinkedPlay { cards, copy });
    Some(Pending::ChooseCards {
        player: res.controller,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::PlayFromExile,
        total: None,
    })
}

pub(super) fn return_cards(
    state: &mut GameState,
    res: &mut Resolution,
    to: ZoneRef,
) -> Option<Pending> {
    let cards = linked(state, res);
    let moves: Vec<_> = cards
        .iter()
        .filter_map(|r| {
            let owner = state.object(r.object)?.owner;
            let destination = match to {
                ZoneRef::Hand => ZoneLocation::Hand(owner),
                ZoneRef::Graveyard => ZoneLocation::Graveyard(owner),
                _ => unreachable!("linked return supports hand and graveyard"),
            };
            Some((r.object, destination))
        })
        .collect();
    if let Some(question) = ask_commander_replace(state, res, &moves) {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement continuation")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::ReturnLinkedExile { to }),
            chosen: Vec::new(),
            asked,
            remaining,
        });
        return Some(question);
    }
    for (card, destination) in moves {
        let _ = state.move_object(card, destination, ZonePosition::Top, Cause::Effect);
    }
    None
}

/// The delayed ability remembers one exact exiled incarnation, without look permission.
pub(super) fn exile_top_until_own_end(state: &mut GameState, res: &Resolution) {
    let Some(&card) = state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .last()
    else {
        return;
    };
    exile(state, res, card, false);
    let Some(object) = state.object(card).filter(|o| o.zone == Zone::Exile) else {
        return;
    };
    let version = object.version;
    state.delayed.push(crate::state::DelayedTrigger {
        controller: res.controller,
        when: crate::state::DelayedWhen::NextYourEndStep,
        action: crate::state::DelayedAction::TriggerAbout {
            source: res.source,
            source_version: source_version(state, res).unwrap_or(0),
            effects: &[baylee_cards_dsl::Effect::ReturnToHand {
                target: baylee_cards_dsl::TargetSpec::EventObject,
            }],
            text: res.text,
            object: card,
            version,
            subject: None,
        },
    });
}

/// A discard trigger cannot chase the card after it leaves the graveyard.
pub(super) fn exile_discarded(state: &mut GameState, res: &Resolution) {
    let Some(reference) = super::subjects::event(state, res) else {
        return;
    };
    if state.object(reference.object).is_none_or(|o| {
        o.version != reference.version
            || o.zone != Zone::Graveyard
            || o.zone_owner != Some(res.controller)
    }) {
        return;
    }
    let _ = state.move_object(
        reference.object,
        ZoneLocation::Exile(res.controller),
        ZonePosition::Top,
        Cause::Effect,
    );
}
