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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_ai::HeuristicAgent;
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use baylee_engine::state::CardLookup;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn agent() -> HeuristicAgent {
        HeuristicAgent::new(AIProfile::default())
    }

    fn island() -> CardIndex {
        baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .expect("registry contains Island")
            .index
    }

    fn forest() -> CardIndex {
        baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index
    }

    fn preset() -> GamePreset {
        let entry = |card: CardIndex| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let seat = |card: CardIndex| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry(card)).collect(),
            sideboard: vec![entry(card), entry(card)],
            commanders: vec![],
            starting_life: None,
            // A hand with something in it, so "was the hand disclosed"
            // is a question an empty list could not answer either way.
            starting_hand: Some(vec![entry(card), entry(card)]),
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 7,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            // Two seats holding different cards, so a report that reached
            // the wrong one says so.
            seats: vec![seat(island()), seat(forest())],
        }
    }

    fn everything() -> ScoutingRequest {
        ScoutingRequest {
            opponents: true,
            hands: true,
            library: LibraryAccess::All,
            sideboards: true,
        }
    }

    fn fixture() -> (Vec<DeckIntel>, GameState) {
        let preset = preset();
        let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");
        (decks(&preset), state)
    }

    /// An AI-chair label is not authority. `Driven` is human-operated and
    /// `StandIn` is a human's chair the house is only holding, so the
    /// *current* controller is checked on every request rather than at the
    /// one moment the seat was set up.
    #[test]
    fn only_a_seat_the_house_is_actually_playing_is_scouted() {
        let (decks, state) = fixture();
        let p0 = PlayerId::new(0);

        for kind in [
            SeatKind::Human,
            SeatKind::Driven(agent()),
            SeatKind::StandIn(agent()),
        ] {
            let seats = [kind, SeatKind::Ai(agent())];
            assert!(
                request(&seats, &decks, &state, p0, everything()).is_none(),
                "a seat somebody is answering for was scouted"
            );
        }

        let seats = [SeatKind::Ai(agent()), SeatKind::Human];
        assert!(request(&seats, &decks, &state, p0, everything()).is_some());
        assert!(
            request(&seats, &decks, &state, PlayerId::new(1), everything()).is_none(),
            "and the human beside it is still a human"
        );
        assert!(
            request(&seats, &decks, &state, PlayerId::new(7), everything()).is_none(),
            "a seat that does not exist is not an AI seat"
        );
    }

    /// Scouting an opponent is what the stronger levels are given; the
    /// ordinary one asks only about the deck it submitted itself.
    #[test]
    fn a_report_reaches_past_its_own_seat_only_when_it_was_asked_to() {
        let (decks, state) = fixture();
        let seats = [SeatKind::Ai(agent()), SeatKind::Ai(agent())];
        let p0 = PlayerId::new(0);

        let own = request(
            &seats,
            &decks,
            &state,
            p0,
            ScoutingRequest {
                opponents: false,
                ..everything()
            },
        )
        .expect("an AI seat");
        assert_eq!(own.seats.len(), 1);
        assert_eq!(own.seats[0].player, p0);
        assert!(
            own.seats[0].deck.cards.iter().all(|c| *c == island()),
            "and it is this seat's own deck"
        );

        let both = request(&seats, &decks, &state, p0, everything()).expect("an AI seat");
        assert_eq!(both.seats.len(), 2);
        assert!(both.seats[1].deck.cards.iter().all(|c| *c == forest()));
    }

    /// Each disclosure is a field of its own, so a level that asks for one
    /// is not handed the rest. The library is the one with a size: `Top(n)`
    /// takes n from the **top**, which is the end of the zone list.
    #[test]
    fn every_disclosure_is_asked_for_separately() {
        let (decks, state) = fixture();
        let seats = [SeatKind::Ai(agent()), SeatKind::Ai(agent())];
        let p0 = PlayerId::new(0);

        let nothing = request(
            &seats,
            &decks,
            &state,
            p0,
            ScoutingRequest {
                opponents: true,
                hands: false,
                library: LibraryAccess::None,
                sideboards: false,
            },
        )
        .expect("an AI seat");
        assert!(nothing.seats[0].hand.is_none());
        assert!(nothing.seats[0].library.is_none());
        assert!(nothing.seats[0].sideboard.is_none());
        assert!(
            !nothing.seats[0].deck.cards.is_empty(),
            "the submitted deck is not a disclosure — it is what a seat \
             hands the table"
        );

        let three = request(
            &seats,
            &decks,
            &state,
            p0,
            ScoutingRequest {
                library: LibraryAccess::Top(3),
                ..everything()
            },
        )
        .expect("an AI seat");
        let library = three.seats[0].library.as_ref().expect("asked for");
        assert_eq!(library.len(), 3);

        let zone = state.zones.list(ZoneLocation::Library(p0));
        let top_first: Vec<CardIndex> = zone
            .iter()
            .rev()
            .take(3)
            .filter_map(|id| state.object(*id).and_then(|o| o.card).map(|c| c.index))
            .collect();
        assert_eq!(*library, top_first, "the top of the library comes first");

        let all = request(&seats, &decks, &state, p0, everything()).expect("an AI seat");
        assert_eq!(
            all.seats[0].library.as_ref().expect("asked for").len(),
            zone.len()
        );
        assert_eq!(all.seats[0].sideboard.as_ref().expect("asked for").len(), 2);
        let hand = all.seats[0].hand.as_ref().expect("asked for");
        assert_eq!(hand.len(), 2);
        assert_eq!(hand.len(), state.zones.list(ZoneLocation::Hand(p0)).len());
    }
}
