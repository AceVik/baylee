//! Private library choices, with explicitly ordered top-card remainders.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn pick_top(
    state: &GameState,
    res: &mut Resolution,
    count: Amount,
    pick: u8,
) -> Option<Pending> {
    let count = amount2(&count, state, res.controller, res) as usize;
    let looked: Vec<_> = state
        .zones
        .list(ZoneLocation::Library(res.controller))
        .iter()
        .rev()
        .take(count)
        .copied()
        .collect();
    if looked.is_empty() {
        return None;
    }
    let pick = usize::from(pick).min(looked.len()) as u8;
    res.awaiting = Some(AwaitingOp::PrivateTopPick {
        looked: looked.clone(),
    });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: looked,
        min: pick,
        max: pick,
        prompt: ChoicePrompt::PutIntoHand,
        total: None,
    })
}

pub(super) fn picked(
    state: &mut GameState,
    res: &mut Resolution,
    looked: Vec<ObjectId>,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let moves: Vec<_> = chosen
        .iter()
        .map(|&card| (card, ZoneLocation::Hand(res.controller)))
        .collect();
    if let Some(pending) = ask_commander_replace(state, res, &moves) {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement");
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::PrivateTopPick { looked }),
            chosen: chosen.to_vec(),
            asked,
            remaining,
        });
        return Some(pending);
    }
    for &card in chosen {
        put_found(state, res.controller, card, SearchDest::Hand);
    }
    let remaining: Vec<_> = looked
        .into_iter()
        .filter(|card| !chosen.contains(card))
        .collect();
    order_top(res, res.controller, remaining)
}

fn order_top(res: &mut Resolution, player: PlayerId, cards: Vec<ObjectId>) -> Option<Pending> {
    if cards.len() < 2 {
        return None;
    }
    let count = cards.len() as u32;
    res.awaiting = Some(AwaitingOp::ReorderTopLibrary { player });
    Some(Pending::Arrange {
        player,
        cards,
        piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, count)],
        prompt: ArrangePrompt::Order,
    })
}

pub(super) fn exile_random_top(
    state: &mut GameState,
    res: &mut Resolution,
    count: u8,
    exile: u8,
) -> Option<Pending> {
    let player = res.controller;
    let mut cards: Vec<_> = state
        .zones
        .list(ZoneLocation::Library(player))
        .iter()
        .rev()
        .take(usize::from(count))
        .copied()
        .collect();
    state.rng.shuffle(&mut cards);
    let exiled = usize::from(exile).min(cards.len());
    for &card in &cards[..exiled] {
        let _ = state.move_object(
            card,
            ZoneLocation::Exile(player),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    let rest = cards[exiled..].to_vec();
    // A single remaining card still needs an acknowledgement to permit the look.
    if rest.len() == 1 {
        res.awaiting = Some(AwaitingOp::InspectHand);
        return Some(Pending::ChooseCards {
            player,
            options: rest,
            min: 0,
            max: 0,
            prompt: ChoicePrompt::LookAtLibrary,
            total: None,
        });
    }
    order_top(res, player, rest)
}

pub(super) fn hand_on_top(
    state: &GameState,
    res: &mut Resolution,
    who: PlayerRel,
    count: u8,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let cards = state.zones.list(ZoneLocation::Hand(player)).clone();
    let count = u32::from(count).min(cards.len() as u32);
    if count == 0 {
        return None;
    }
    res.awaiting = Some(AwaitingOp::HandToTopChoice { player });
    Some(Pending::ChooseCards {
        player,
        options: cards,
        min: count as u8,
        max: count as u8,
        prompt: ChoicePrompt::PutBackOnTop,
        total: None,
    })
}

pub(super) fn chosen_hand_to_top(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if chosen.len() < 2 {
        return hand_transfer::place(state, res, player, chosen);
    }
    res.awaiting = Some(AwaitingOp::HandToTop { player });
    Some(Pending::Arrange {
        player,
        cards: chosen.to_vec(),
        piles: vec![ArrangePile::all_of(
            ArrangePlace::LibraryTop,
            chosen.len() as u32,
        )],
        prompt: ArrangePrompt::Order,
    })
}
