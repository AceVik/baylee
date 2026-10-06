use super::*;

#[test]
fn scouting_is_revoked_on_takeover_and_never_granted_to_human_standins() {
    use baylee_ai::intelligence::{LibraryAccess, ScoutingRequest};
    let mut preset = split_preset();
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Ai(AIProfile::EXPERT);
    preset.seats[1].sideboard = vec![DeckEntry {
        card: forest(),
        print: PrintRef::new(1),
    }];
    let mut session = Session::new(&preset).unwrap();
    let request = ScoutingRequest {
        opponents: true,
        hands: true,
        library: LibraryAccess::All,
        sideboards: true,
    };
    let scout = |session: &Session, player| {
        crate::scouting::request(
            &session.seats,
            &session.scouting_decks,
            session.engine.state(),
            player,
            request,
        )
        .is_some()
    };
    assert!(scout(&session, PlayerId::new(1)));
    assert!(!scout(&session, PlayerId::new(0)));
    assert!(!scout(&session, PlayerId::new(250)));
    let before = serde_json::to_vec(&session.game_static(PlayerId::new(0))).unwrap();
    let report = crate::scouting::request(
        &session.seats,
        &session.scouting_decks,
        session.engine.state(),
        PlayerId::new(1),
        request,
    )
    .unwrap();
    assert_eq!(report.seats.len(), 2);
    for seat in &report.seats {
        let p = usize::from(seat.player.get());
        assert_eq!(
            seat.deck.cards,
            preset.seats[p]
                .deck
                .iter()
                .map(|e| e.card)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            seat.library.as_ref().unwrap().len(),
            session
                .engine
                .state()
                .zones
                .list(baylee_engine::zone::ZoneLocation::Library(seat.player))
                .len()
        );
    }
    assert_eq!(
        report.seats[1].sideboard.as_deref(),
        Some([forest()].as_slice())
    );
    assert_eq!(
        serde_json::to_vec(&session.game_static(PlayerId::new(0))).unwrap(),
        before,
        "scouting must not teach human sockets any print identity"
    );
    assert!(session.take_over(PlayerId::new(1)));
    assert!(!scout(&session, PlayerId::new(1)));
    assert!(session.release(PlayerId::new(1)));
    assert!(scout(&session, PlayerId::new(1)));
    assert!(session.stand_in(PlayerId::new(0)));
    assert!(!scout(&session, PlayerId::new(0)));
    assert!(session.hand_back(PlayerId::new(0)));
    assert!(!scout(&session, PlayerId::new(0)));
}

#[test]
fn scouting_top_cards_are_bounded_ordered_and_do_not_change_human_views() {
    use baylee_ai::intelligence::{LibraryAccess, ScoutingRequest};
    let mut preset = test_preset();
    preset.seats[1].deck[2].card = forest();
    preset.seats[1].deck[8].card = forest();
    let session = Session::new(&preset).unwrap();
    let before = crate::view::player_view(
        session.engine.state(),
        PlayerId::new(0),
        0,
        None,
        &crate::view::SeatContext::default(),
        &[],
    );
    let report = crate::scouting::request(
        &session.seats,
        &session.scouting_decks,
        session.engine.state(),
        PlayerId::new(1),
        ScoutingRequest {
            library: LibraryAccess::Top(3),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.seats.len(), 1);
    assert!(report.seats[0].hand.is_none());
    assert!(report.seats[0].sideboard.is_none());
    let expected: Vec<_> = session
        .engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Library(PlayerId::new(1)))
        .iter()
        .rev()
        .take(3)
        .map(|id| {
            session
                .engine
                .state()
                .object(*id)
                .unwrap()
                .card
                .unwrap()
                .index
        })
        .collect();
    assert_eq!(report.seats[0].library.as_ref().unwrap(), &expected);
    assert_eq!(
        before,
        crate::view::player_view(
            session.engine.state(),
            PlayerId::new(0),
            0,
            None,
            &crate::view::SeatContext::default(),
            &[]
        )
    );
}
