//! Private, in-process AI access. There is deliberately no protocol message,
//! serialization implementation or public Session getter for this data.

use baylee_ai::intelligence::{
    DeckIntel, LibraryAccess, ScoutedSeat, ScoutingReport, ScoutingRequest,
};
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::GamePreset;
use baylee_engine::state::GameState;
use baylee_engine::zone::ZoneLocation;

use crate::SeatKind;

pub(crate) fn decks(preset: &GamePreset) -> Vec<DeckIntel> {
    preset
        .seats
        .iter()
        .map(|s| {
            DeckIntel::new(
                s.deck.iter().map(|e| e.card).collect(),
                s.commanders.iter().map(|e| e.card).collect(),
            )
        })
        .collect()
}

/// Check the *current* controller on every request. An AI-chair label is not
/// authority: `Driven` is human-operated, and `StandIn` still belongs to a human.
pub(crate) fn request<'a>(
    seats: &[SeatKind],
    decks: &'a [DeckIntel],
    state: &GameState,
    player: PlayerId,
    request: ScoutingRequest,
) -> Option<ScoutingReport<'a>> {
    if !matches!(seats.get(usize::from(player.get())), Some(SeatKind::Ai(_))) {
        return None;
    }
    let cards = |zone: ZoneLocation, limit: usize| -> Vec<CardIndex> {
        state
            .zones
            .list(zone)
            .iter()
            .rev()
            .take(limit)
            .filter_map(|id| state.object(*id).and_then(|o| o.card).map(|c| c.index))
            .collect()
    };
    Some(ScoutingReport {
        seats: decks
            .iter()
            .enumerate()
            .filter_map(|(i, deck)| {
                let seat = PlayerId::new(u8::try_from(i).ok()?);
                if seat != player && !request.opponents {
                    return None;
                }
                Some(ScoutedSeat {
                    player: seat,
                    deck,
                    hand: request
                        .hands
                        .then(|| cards(ZoneLocation::Hand(seat), usize::MAX)),
                    library: match request.library {
                        LibraryAccess::None => None,
                        LibraryAccess::Top(n) => {
                            Some(cards(ZoneLocation::Library(seat), usize::from(n)))
                        }
                        LibraryAccess::All => Some(cards(ZoneLocation::Library(seat), usize::MAX)),
                    },
                    sideboard: request
                        .sideboards
                        .then(|| cards(ZoneLocation::OutsideGame(seat), usize::MAX)),
                })
            })
            .collect(),
    })
}
