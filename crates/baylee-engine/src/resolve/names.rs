//! Resolution-time naming and operations on the cards bearing that name.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::object::PrintedFace;
use baylee_core::ids::NameRef;

/// Continue after the driver validates an Oracle face and interns its name.
pub fn resume_name(
    state: &mut GameState,
    res: &mut Resolution,
    printed: PrintedFace,
    name: NameRef,
) -> Flow {
    let Some(AwaitingOp::ChooseCardName { remember }) = res.awaiting.take() else {
        unreachable!("name choice continuation")
    };
    res.subject.named = Some((printed, name));
    if remember
        && let Some(source) =
            subjects::source(state, res).filter(|source| subjects::on_battlefield(state, *source))
    {
        let acquisition = super::subjects::acquisition(state, res);
        if let Some(object) = state.object_mut(source.object) {
            object.remember_name(acquisition, printed, name);
        }
        state.invalidate_projections();
    }
    advance_program(res);
    run(state, res)
}

fn matches_name(state: &GameState, res: &Resolution, card: ObjectId) -> bool {
    let Some((printed, name)) = res.subject.named else {
        return false;
    };
    state.object(card).is_some_and(|object| {
        object.characteristics().name == name
            // Rooms have both half names outside the stack and battlefield.
            || (object.doors.is_room()
                && object.zone != crate::zone::Zone::Battlefield
                && object.zone != crate::zone::Zone::Stack
                && object.card.is_some_and(|card| card.index == printed.card()))
    })
}

pub(super) fn exile_top(
    state: &mut GameState,
    res: &Resolution,
    who: PlayerRel,
    count: Amount,
) -> Option<Pending> {
    let count = amount2(&count, state, res.controller, res) as usize;
    for player in players_of(who, state, res.controller, res) {
        let cards: Vec<_> = state
            .zones
            .list(ZoneLocation::Library(player))
            .iter()
            .rev()
            .take(count)
            .copied()
            .collect();
        for card in cards {
            put_found(state, player, card, SearchDest::Exile);
        }
    }
    None
}

pub(super) fn reveal_until_named(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    matched: SearchDest,
    otherwise: SearchDest,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let cards = state.zones.list(ZoneLocation::Library(player)).clone();
    for card in cards.into_iter().rev() {
        let matches = matches_name(state, res, card);
        state.journal.record(GameEvent::Revealed {
            player,
            cards: vec![card],
        });
        if matches {
            return put_top(state, res, card, player, matched);
        }
        put_found(state, player, card, otherwise);
    }
    None
}

pub(super) fn reveal_top_then(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    matched: SearchDest,
    otherwise: SearchDest,
    otherwise_effects: &'static [Effect],
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let card = *state.zones.list(ZoneLocation::Library(player)).last()?;
    let matches = matches_name(state, res, card);
    state.journal.record(GameEvent::Revealed {
        player,
        cards: vec![card],
    });
    if !matches {
        let next = res.pc + 1;
        res.effects.splice(next..next, flatten(otherwise_effects));
    }
    put_top(
        state,
        res,
        card,
        player,
        if matches { matched } else { otherwise },
    )
}

pub(super) fn reveal_top(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerRel,
    matched: SearchDest,
    otherwise: SearchDest,
) -> Option<Pending> {
    let player = players_of(player, state, res.controller, res)
        .first()
        .copied()?;
    let card = *state.zones.list(ZoneLocation::Library(player)).last()?;
    let destination = if matches_name(state, res, card) {
        matched
    } else {
        otherwise
    };
    state.journal.record(GameEvent::Revealed {
        player,
        cards: vec![card],
    });
    put_top(state, res, card, player, destination)
}

pub(super) fn put_top(
    state: &mut GameState,
    res: &mut Resolution,
    card: ObjectId,
    player: PlayerId,
    destination: SearchDest,
) -> Option<Pending> {
    if destination == SearchDest::Hand
        && let Some(question) =
            ask_commander_replace(state, res, &[(card, ZoneLocation::Hand(player))])
    {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement owns its continuation")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::NamedTop {
                card,
                player,
                destination,
            }),
            chosen: vec![card],
            asked,
            remaining,
        });
        return Some(question);
    }
    put_found(state, player, card, destination);
    None
}

pub(super) fn random_discard(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerRel,
    count: Amount,
) -> Option<Pending> {
    let player = players_of(player, state, res.controller, res)
        .first()
        .copied()?;
    let count = usize::try_from(amount2(&count, state, res.controller, res)).unwrap_or(usize::MAX);
    let mut hand = state.zones.list(ZoneLocation::Hand(player)).clone();
    if count == 0 || hand.is_empty() {
        return None;
    }
    state.rng.shuffle(&mut hand);
    hand.truncate(count);
    state.journal.record(GameEvent::Revealed {
        player,
        cards: hand.clone(),
    });
    let discards: Vec<_> = hand
        .into_iter()
        .filter(|card| matches_name(state, res, *card))
        .map(|card| (card, player))
        .collect();
    discard_or_ask(state, res, &discards)
}
