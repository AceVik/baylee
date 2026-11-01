//! A hand's owner orders its simultaneous movement onto their library.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn all_to_top(
    state: &GameState,
    res: &mut Resolution,
    who: PlayerRel,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let cards = state.zones.list(ZoneLocation::Hand(player)).clone();
    if cards.is_empty() {
        return None;
    }
    let count = cards.len() as u32;
    res.awaiting = Some(AwaitingOp::HandToTop { player });
    Some(Pending::Arrange {
        player,
        cards,
        piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, count)],
        prompt: ArrangePrompt::Order,
    })
}

pub(super) fn place(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    top: &[ObjectId],
) -> Option<Pending> {
    let moves: Vec<_> = top
        .iter()
        .copied()
        .map(|card| (card, ZoneLocation::Library(player)))
        .collect();
    if let Some(pending) = ask_commander_replace(state, res, &moves) {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement");
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::HandToTopPlace { player }),
            chosen: top.to_vec(),
            asked,
            remaining,
        });
        return Some(pending);
    }
    for &card in top.iter().rev() {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(player),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    None
}
