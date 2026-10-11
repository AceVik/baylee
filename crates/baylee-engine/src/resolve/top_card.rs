//! Private single-card inspections and non-targeted command-zone selections.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{Filter, LookRest};

pub(super) fn start(
    state: &GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    rest: LookRest,
    then: &'static [Effect],
) -> Option<Pending> {
    let &card = state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .last()?;
    let matches = state.object(card).is_some_and(|o| {
        eval::matches_with_context(filter, state, o, res.controller, res.rule_context())
    });
    res.awaiting = Some(AwaitingOp::TakeTop {
        looked: [card],
        rest,
        reveal: true,
        then,
    });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: if matches { vec![card] } else { vec![] },
        min: 0,
        max: u8::from(matches),
        prompt: ChoicePrompt::PutIntoHand,
        total: None,
    })
}

pub(super) fn take(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[ObjectId],
    looked: [ObjectId; 1],
    rest: LookRest,
    reveal: bool,
    then: &'static [Effect],
) -> Option<Pending> {
    let card = looked[0];
    if !state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .contains(&card)
    {
        return None;
    }
    if chosen.contains(&card) {
        if reveal {
            state.journal.record(GameEvent::Revealed {
                player: res.controller,
                cards: vec![card],
            });
        }
        if let Some(question) =
            ask_commander_replace(state, res, &[(card, ZoneLocation::Hand(res.controller))])
        {
            let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take()
            else {
                unreachable!("replacement retains owners")
            };
            res.awaiting = Some(AwaitingOp::SelectedCommander {
                operation: Box::new(AwaitingOp::TakeTop {
                    looked,
                    rest,
                    reveal: false,
                    then,
                }),
                chosen: chosen.to_vec(),
                asked,
                remaining,
            });
            return Some(question);
        }
        put_found(state, res.controller, card, SearchDest::Hand);
        if state
            .zones
            .list(ZoneLocation::Hand(res.controller))
            .contains(&card)
        {
            let next = res.pc + 1;
            res.effects.splice(next..next, then.iter().copied());
        }
        return None;
    }
    if rest == LookRest::Keep {
        return None;
    }
    res.awaiting = Some(AwaitingOp::RestTop { looked, rest });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: vec![card],
        min: 0,
        max: 1,
        prompt: if rest == LookRest::MayBottom {
            ChoicePrompt::PutOnBottom
        } else {
            ChoicePrompt::PutIntoGraveyard
        },
        total: None,
    })
}

pub(super) fn rest(
    state: &mut GameState,
    res: &Resolution,
    chosen: &[ObjectId],
    looked: [ObjectId; 1],
    rest: LookRest,
) {
    let card = looked[0];
    let library = ZoneLocation::Library(res.controller);
    if !chosen.contains(&card) || !state.zones.list(library).contains(&card) {
        return;
    }
    match rest {
        LookRest::Keep => {}
        LookRest::MayBottom => {
            // CR 400.7: reordering within one zone never creates a new object.
            state.zones.remove(card, library);
            state
                .zones
                .insert(card, library, ZonePosition::Bottom, true);
        }
        LookRest::MayGraveyard => {
            let _ = state.move_object(
                card,
                ZoneLocation::Graveyard(res.controller),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }
}

pub(super) fn commander(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let options: Vec<_> = state
        .zones
        .list(ZoneLocation::Command(res.controller))
        .iter()
        .copied()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.owner == res.controller
                    && state.commanders.iter().flatten().any(|c| c.object == *id)
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::CommanderToHand);
    Some(Pending::ChooseCards {
        player: res.controller,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::PutIntoHand,
        total: None,
    })
}

pub(super) fn take_commander(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let &card = chosen.first()?;
    if let Some(question) =
        ask_commander_replace(state, res, &[(card, ZoneLocation::Hand(res.controller))])
    {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("replacement retains owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::CommanderToHand),
            chosen: chosen.to_vec(),
            asked,
            remaining,
        });
        return Some(question);
    }
    if let Some(at) = state
        .commander_redirect
        .iter()
        .position(|(id, home)| *id == card && *home)
    {
        // It is already in the command zone. Choosing to keep it there is
        // not a zone change and must preserve this incarnation (CR 400.7).
        state.commander_redirect.remove(at);
    } else {
        let _ = state.move_object(
            card,
            ZoneLocation::Hand(res.controller),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    None
}

/// Viewing is a private acknowledgement, never a public reveal or a reorder.
pub(super) fn inspect(
    state: &GameState,
    res: &mut Resolution,
    who: PlayerRel,
    count: u8,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let options: Vec<_> = state
        .zones
        .list(ZoneLocation::Library(player))
        .iter()
        .rev()
        .take(usize::from(count))
        .copied()
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::InspectHand);
    Some(Pending::ChooseCards {
        player: res.controller,
        options,
        min: 0,
        max: 0,
        prompt: ChoicePrompt::LookAtLibrary,
        total: None,
    })
}

pub(super) fn reveal_if(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
    find: baylee_cards_dsl::Find,
) {
    let player = res.controller;
    let Some(&card) = state.zones.list(ZoneLocation::Library(player)).last() else {
        return;
    };
    state.journal.record(GameEvent::Revealed {
        player,
        cards: vec![card],
    });
    if !state
        .object(card)
        .is_some_and(|o| eval::matches_with_context(filter, state, o, player, res.rule_context()))
    {
        return;
    }
    // The card remains in place on a nonmatch, rather than being moved to the same zone.
    if find.tapped && find.dest == SearchDest::Battlefield {
        state.restore_tapped(card, true);
    }
    put_found(state, player, card, find.dest);
    if let Some((kind, n)) = find.counter {
        crate::entry::put_counters(state, player, card, kind, n);
    }
}
