//! Shared choices and completion for effects that inspect a library's top.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// What happens to the inspected cards which were not selected.
#[derive(Clone, Copy, Debug)]
pub enum RestDestination {
    /// Put them on the bottom, in the printed random or chosen order.
    Bottom {
        /// "In a random order" rather than the player's.
        random: bool,
    },
    /// Move each one to this destination without an ordering question.
    Move(SearchDest),
}

/// Inspect up to `count` cards and offer at most one matching card.
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    count: u8,
    filter: &'static baylee_cards_dsl::Filter,
    found: baylee_cards_dsl::Find,
    reveal: bool,
    destination: RestDestination,
) -> Option<Pending> {
    let you = res.controller;
    let top: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(you))
        .iter()
        .rev()
        .take(usize::from(count))
        .copied()
        .collect();
    if top.is_empty() {
        return None;
    }
    if reveal {
        state.journal.record(GameEvent::Revealed {
            player: you,
            cards: top.clone(),
        });
    }
    let options: Vec<ObjectId> = top
        .iter()
        .copied()
        .filter(|id| {
            state.object(*id).is_some_and(|object| {
                eval::matches_with_context(filter, state, object, you, res.rule_context())
            })
        })
        .collect();
    if options.is_empty() {
        return finish_with_destination(state, res, found, top, false, destination, &[]);
    }
    res.awaiting = Some(AwaitingOp::DigRest {
        rest: top,
        reveal: !reveal,
        destination,
        found,
    });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: 0,
        max: 1,
        prompt: match found.dest {
            SearchDest::Hand => ChoicePrompt::PutIntoHand,
            SearchDest::Battlefield => ChoicePrompt::PutOntoBattlefield,
            SearchDest::TopOfLibrary => ChoicePrompt::PutBackOnTop,
            SearchDest::Graveyard | SearchDest::Exile => ChoicePrompt::Generic,
        },
        total: None,
    })
}

/// Moves selected cards, then orders the rest on the library's bottom.
pub(super) fn finish(
    state: &mut GameState,
    res: &mut Resolution,
    found: baylee_cards_dsl::Find,
    rest: Vec<ObjectId>,
    reveal: bool,
    random: bool,
    chosen: &[ObjectId],
) -> Option<Pending> {
    finish_with_destination(
        state,
        res,
        found,
        rest,
        reveal,
        RestDestination::Bottom { random },
        chosen,
    )
}

/// The same destination `put_found` will use, before anything moves.
fn find_zone(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    destination: SearchDest,
) -> ZoneLocation {
    let owner = state.object(card).map_or(player, |object| object.owner);
    match destination {
        SearchDest::Graveyard => ZoneLocation::Graveyard(owner),
        SearchDest::Exile => ZoneLocation::Exile(owner),
        SearchDest::Hand => ZoneLocation::Hand(player),
        SearchDest::TopOfLibrary => ZoneLocation::Library(player),
        SearchDest::Battlefield => ZoneLocation::Battlefield,
    }
}

/// Moves selected cards, then completes their remainder as specified.
pub(super) fn finish_with_destination(
    state: &mut GameState,
    res: &mut Resolution,
    found: baylee_cards_dsl::Find,
    rest: Vec<ObjectId>,
    reveal: bool,
    destination: RestDestination,
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
    let mut moves: Vec<_> = placed
        .iter()
        .map(|&(card, find)| (card, find_zone(state, res.controller, card, find.dest)))
        .collect();
    let remaining: Vec<ObjectId> = rest
        .iter()
        .copied()
        .filter(|c| !chosen.contains(c))
        .collect();
    if let RestDestination::Move(destination) = destination {
        moves.extend(
            remaining
                .iter()
                .map(|&card| (card, find_zone(state, res.controller, card, destination))),
        );
    }
    // Every owner answers before any selected or unselected card moves. The
    // last answer re-enters this function and therefore plans the same moves.
    if let Some(pending) = ask_commander_replace(state, res, &moves) {
        let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take() else {
            unreachable!("replacement question retains its owners")
        };
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation: Box::new(AwaitingOp::DigRest {
                found,
                rest,
                reveal: false,
                destination,
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
    match destination {
        RestDestination::Move(destination) => {
            for card in remaining {
                put_found(state, res.controller, card, destination);
            }
        }
        RestDestination::Bottom { random } => {
            let mut remaining = remaining;
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
        }
    }
    None
}
