//! Shared completion for digging effects whose unpicked cards go on the bottom.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// Moves selected cards, then asks for the order of the rest when necessary.
pub(super) fn finish(
    state: &mut GameState,
    res: &mut Resolution,
    found: baylee_cards_dsl::Find,
    rest: Vec<ObjectId>,
    reveal: bool,
    random: bool,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if reveal && !chosen.is_empty() {
        state.journal.record(GameEvent::Revealed {
            player: res.controller,
            cards: chosen.to_vec(),
        });
    }
    let placed: Vec<_> = chosen
        .iter()
        .map(|&card| {
            (
                card,
                search::resolve_find(state, res, res.controller, found, card),
            )
        })
        .collect();
    let moves: Vec<_> = placed
        .iter()
        .map(|&(card, find)| {
            let destination = match find.dest {
                SearchDest::Graveyard => ZoneLocation::Graveyard(res.controller),
                SearchDest::Exile => {
                    ZoneLocation::Exile(state.object(card).map_or(res.controller, |o| o.owner))
                }
                SearchDest::Hand => ZoneLocation::Hand(res.controller),
                SearchDest::TopOfLibrary => ZoneLocation::Library(res.controller),
                SearchDest::Battlefield => ZoneLocation::Battlefield,
            };
            (card, destination)
        })
        .collect();
    if let Some(pending) = ask_commander_replace(state, res, &moves) {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("replacement question retains its owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::DigRest {
                found,
                rest,
                reveal: false,
                random,
            }),
            chosen: chosen.to_vec(),
            asked,
            remaining,
        });
        return Some(pending);
    }
    for (card, find) in placed {
        if find.tapped && find.dest == SearchDest::Battlefield {
            state.restore_tapped(card, true);
        }
        put_found(state, res.controller, card, find.dest);
        if find.dest == SearchDest::Battlefield
            && let Some((kind, count)) = find.counter
        {
            crate::entry::put_counters(state, res.controller, card, kind, count);
        }
    }
    let mut remaining: Vec<ObjectId> = rest.into_iter().filter(|c| !chosen.contains(c)).collect();
    if random {
        state.rng.shuffle(&mut remaining);
    }
    if remaining.len() > 1 && !random {
        res.awaiting = Some(AwaitingOp::DigBottom);
        let n = u32::try_from(remaining.len()).unwrap_or(u32::MAX);
        return Some(Pending::Arrange {
            player: res.controller,
            cards: remaining,
            piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
            prompt: ArrangePrompt::Order,
        });
    }
    for card in remaining {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(res.controller),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
    None
}
