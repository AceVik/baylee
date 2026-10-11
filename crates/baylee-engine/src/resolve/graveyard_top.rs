//! Ordered graveyard targets, with command-zone choices before any card moves.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn begin(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let player = res.target_players.iter().next()?;
    let cards: Vec<_> = res
        .second_targets
        .iter()
        .copied()
        .filter(|&card| {
            state.object(card).is_some_and(|object| {
                object.zone == crate::zone::Zone::Graveyard && object.owner == player
            })
        })
        .collect();
    if cards.len() <= 1 {
        return place(state, res, player, cards);
    }
    let count = u32::try_from(cards.len()).unwrap_or(u32::MAX);
    res.awaiting = Some(AwaitingOp::GraveyardToTopOrder { player });
    Some(Pending::Arrange {
        player: res.controller,
        cards,
        piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, count)],
        prompt: ArrangePrompt::Order,
    })
}

pub(super) fn place(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    cards: Vec<ObjectId>,
) -> Option<Pending> {
    let moves: Vec<_> = cards
        .iter()
        .map(|&card| (card, ZoneLocation::Library(player)))
        .collect();
    if let Some(question) = ask_commander_replace(state, res, &moves) {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("commander replacement continuation")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::GraveyardToTopOrdered {
                player,
                cards: cards.clone(),
            }),
            chosen: cards,
            asked,
            remaining,
        });
        return Some(question);
    }
    for card in cards.into_iter().rev() {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(player),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    None
}
