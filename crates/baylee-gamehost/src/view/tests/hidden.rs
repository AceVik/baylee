use super::*;

/// Every object id that appears anywhere in a view.
fn ids_in(view: &baylee_view::PlayerView) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
    for zone in [&view.battlefield, &view.stack] {
        ids.extend(zone.iter().map(|o| o.id));
    }
    for per_seat in [&view.graveyards, &view.exile, &view.command] {
        for zone in per_seat {
            ids.extend(zone.iter().map(|o| o.id));
        }
    }
    ids.extend(view.library_tops.iter().map(|o| o.id));
    ids.extend(view.combat.attackers.iter().map(|a| a.creature));
    ids.extend(view.combat.blockers.iter().map(|b| b.blocker));
    ids.extend(view.combat.bands.iter().flatten());
    ids
}

/// The opponent's hand is a number. Not a list the client is trusted to
/// hide, not ids with the names stripped — a count, with no field the
/// contents could travel in.
#[test]
fn an_opponents_hand_is_a_count_and_nothing_else() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let view = player_view(engine.state(), me, 1, None, &SeatContext::default(), &[]);

    let their_hand = engine.state().zones.list(ZoneLocation::Hand(them));
    assert!(!their_hand.is_empty(), "the opponent holds cards");
    assert_eq!(
        view.seat(them).map(|s| s.hand_count),
        Some(their_hand.len() as u32),
        "the count is what a client gets"
    );
    let visible = ids_in(&view);
    for id in their_hand {
        assert!(
            !visible.contains(id),
            "an opponent's hand card reached seat 0's view: {id:?}"
        );
    }
}

/// Nobody's library is in the view — not even the viewing seat's own.
/// A player who could read their own library order would know every
/// draw, which is the same leak wearing a friendlier hat.
#[test]
fn no_library_card_reaches_any_view() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        let view = player_view(engine.state(), seat, 1, None, &SeatContext::default(), &[]);
        let visible = ids_in(&view);
        for owner in [PlayerId::new(0), PlayerId::new(1)] {
            let library = engine.state().zones.list(ZoneLocation::Library(owner));
            assert!(
                library.len() > 40,
                "the library should still be nearly whole"
            );
            for id in library {
                assert!(
                    !visible.contains(id),
                    "a library card reached {seat:?}'s view: {id:?}"
                );
            }
            assert_eq!(
                view.seat(owner).map(|s| s.library_count),
                Some(library.len() as u32)
            );
        }
    }
}

/// A two-seat table dealt seven cards each, with the mulligans open.
fn a_table_deciding_its_mulligans() -> Engine<Registry> {
    let mut preset = mixed_print_preset();
    for seat in &mut preset.seats {
        seat.starting_hand = None;
        seat.starting_battlefield = vec![];
    }
    Engine::new(&preset, Registry).expect("game starts")
}

/// Before turn 1 every seat is asked its own mulligan at once (#257), so
/// a view waits on its own seat while that seat is deciding, and on
/// nobody once it has kept. From turn 1 on every view waits on the same
/// seat again, and `deciding` is empty: it is the window.
#[test]
fn during_the_mulligans_each_view_waits_on_its_own_seat() {
    use baylee_engine::choice::PlayerAction;

    let mut engine = a_table_deciding_its_mulligans();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let both: SeatSet = [me, them].into_iter().collect();
    for seat in [me, them] {
        let view = seen_by(&engine, seat);
        assert_eq!(view.awaiting, Some(seat));
        assert_eq!(view.deciding, both);
    }

    engine.apply(me, PlayerAction::MulliganKeep).unwrap();
    let mine = seen_by(&engine, me);
    let theirs = seen_by(&engine, them);
    assert_eq!(mine.awaiting, None, "a seat that has kept is asked nothing");
    assert_eq!(theirs.awaiting, Some(them));
    let still: SeatSet = [them].into_iter().collect();
    assert_eq!((mine.deciding, theirs.deciding), (still, still));

    engine.apply(them, PlayerAction::MulliganKeep).unwrap();
    let asked = engine.pending().asked();
    assert!(asked.is_some(), "turn 1 asks somebody");
    for seat in [me, them] {
        let view = seen_by(&engine, seat);
        assert_eq!(view.deciding, SeatSet::new(), "the window has closed");
        assert_eq!(view.awaiting, asked, "every view waits on the same seat");
    }
}

/// Another seat's mulligans reach this seat's view as `deciding` and as
/// that seat's counts, and as nothing else: not the question it is
/// answering, not the hands it drew, not the cards it put on the bottom.
#[test]
fn another_seats_mulligan_shows_only_as_deciding_and_its_counts() {
    use baylee_engine::choice::PlayerAction;

    let mut engine = a_table_deciding_its_mulligans();
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let before = seen_by(&engine, me);

    // Two takes: a new hand each time, and a bottom owed however the
    // first mulligan is priced.
    engine.apply(them, PlayerAction::MulliganTake).unwrap();
    engine.apply(them, PlayerAction::MulliganTake).unwrap();
    assert_eq!(seen_by(&engine, me), before, "a take shows nothing here");
    engine.apply(them, PlayerAction::MulliganKeep).unwrap();
    let Some(Pending::MulliganBottom { count, .. }) = engine.pending_for(them).cloned() else {
        panic!("two takes owe a bottom");
    };
    assert_eq!(
        seen_by(&engine, me),
        before,
        "their bottom question shows nothing here"
    );

    let hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
    let objects = hand.iter().take(usize::from(count)).copied().collect();
    engine
        .apply(them, PlayerAction::ChooseObjects { objects })
        .unwrap();
    let mut after = seen_by(&engine, me);
    let line = after.seat(them).expect("their seat line");
    assert_eq!(line.hand_count, 7 - u32::from(count));
    assert_eq!(after.deciding, [me].into_iter().collect());
    // Put back the three things that may differ, and nothing else did.
    after.deciding = before.deciding;
    after.seats[1].hand_count = before.seats[1].hand_count;
    after.seats[1].library_count = before.seats[1].library_count;
    assert_eq!(after, before);
}

/// Two seats looking at the same battlefield see different things when a
/// permanent is face down (CR 708.5): its controller knows what they
/// played, everyone else gets a blank with no card identity at all.
#[test]
fn a_face_down_permanent_is_blank_to_everyone_but_its_controller() {
    let preset = mixed_print_preset();
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    // The test preset grants seat 0 dev commands; a lobby game grants
    // nobody any, which is what makes this the harness and not a hole.
    engine
        .dev_state_mut(me)
        .expect("the test preset grants dev commands")
        .object_mut(land)
        .expect("the permanent is there")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);

    let mine = player_view(engine.state(), me, 1, None, &SeatContext::default(), &[]);
    let theirs = player_view(engine.state(), them, 1, None, &SeatContext::default(), &[]);
    let of = |v: &baylee_view::PlayerView| {
        v.battlefield
            .iter()
            .find(|o| o.id == land)
            .expect("the permanent is on the shared battlefield")
            .clone()
    };
    assert!(
        of(&mine).card.is_some(),
        "its controller knows what they played"
    );
    assert!(
        of(&theirs).card.is_none(),
        "the opponent was handed the card identity of a face-down permanent"
    );
    assert_eq!(of(&theirs).name, "Face-down");
}

/// A search offered to `player`, over `options`.
fn search(player: PlayerId, options: Vec<ObjectId>) -> Pending {
    Pending::ChooseCards {
        player,
        options,
        min: 1,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
        total: None,
    }
}

/// A tutor hands a seat object ids out of its own library. Every other
/// zone the client can draw from is in the view already; these are in
/// none of them, so without `looking_at` the dialog is a row of blanks
/// and the choice cannot be answered at all.
#[test]
fn a_searching_seat_is_shown_the_cards_it_was_offered() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let seat = PlayerId::new(0);
    let offered = library(&engine, seat, 3);
    let pending = search(seat, offered.clone());

    let view = player_view(
        engine.state(),
        seat,
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    let shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
    assert_eq!(
        shown, offered,
        "the searcher was not shown what it was asked about"
    );
    assert!(
        view.looking_at.iter().all(|o| o.card.is_some()),
        "a card offered out of a library arrived without its identity"
    );
}

/// The entitlement is the question, not the game state: the seat being
/// asked sees the search, and the table does not. This is the sentence
/// the whole field rests on, so it is the one with a test.
#[test]
fn nobody_else_is_shown_another_seat_s_search() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let searcher = PlayerId::new(0);
    let pending = search(searcher, library(&engine, searcher, 3));

    let theirs = player_view(
        engine.state(),
        PlayerId::new(1),
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    assert!(
        theirs.looking_at.is_empty(),
        "an opponent was shown the cards a searching seat is looking through"
    );
}

/// Fact or Fiction's two questions name cards in the caster's library:
/// the opponent who separates them is asked about another seat's
/// library, and the caster then about piles. Each is shown the revealed
/// cards with their identity while asked, or the piles are blanks.
#[test]
fn a_separator_and_a_pile_chooser_are_shown_the_revealed_cards() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let caster = PlayerId::new(0);
    let separator = PlayerId::new(1);
    let revealed = library(&engine, caster, 3);
    let shown = |seat: PlayerId, pending: &Pending| {
        let view = player_view(
            engine.state(),
            seat,
            0,
            Some(pending),
            &SeatContext::default(),
            &[],
        );
        assert!(
            view.looking_at.iter().all(|o| o.card.is_some()),
            "a revealed card arrived without its identity"
        );
        view.looking_at.iter().map(|o| o.id).collect::<Vec<_>>()
    };

    let separate = Pending::ChooseCards {
        player: separator,
        options: revealed.clone(),
        min: 0,
        max: 3,
        prompt: baylee_engine::choice::ChoicePrompt::FirstPile,
        total: None,
    };
    assert_eq!(shown(separator, &separate), revealed);

    let choose = Pending::ChoosePile {
        player: caster,
        piles: vec![vec![revealed[1]], vec![revealed[0], revealed[2]]],
    };
    assert_eq!(
        shown(caster, &choose),
        vec![revealed[1], revealed[0], revealed[2]],
        "every pile's cards, in pile order"
    );
    assert!(
        shown(separator, &choose).is_empty(),
        "the pile question is the caster's alone"
    );
}

/// And it is not a memory. The list is rebuilt from the outstanding
/// choice every time, so the moment the question is gone the cards are
/// gone with it — there is nowhere for one to linger.
#[test]
fn a_card_stops_being_shown_when_the_question_ends() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let seat = PlayerId::new(0);

    let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);
    assert!(
        view.looking_at.is_empty(),
        "a view with no pending choice was still showing cards"
    );
}

/// Most choices name things that are already on the table, and those must
/// not arrive twice: a client that drew `looking_at` as a dialog would
/// open one over an ordinary "target creature".
#[test]
fn an_offer_of_things_already_in_view_shows_nothing_extra() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let seat = PlayerId::new(0);
    let battlefield: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    assert!(!battlefield.is_empty(), "the preset seats a battlefield");
    let pending = Pending::ChooseTargets {
        player: seat,
        options: battlefield,
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };

    let view = player_view(
        engine.state(),
        seat,
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    assert!(
        view.looking_at.is_empty(),
        "objects the view already carries were repeated as things being shown"
    );
}

/// A seat earns a printing by seeing the card, and a card out of a
/// library is a card it now sees. Without this the print table has no
/// entry for it and the dialog draws rectangles.
#[test]
fn a_card_being_shown_earns_its_printing() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let seat = PlayerId::new(0);
    let pending = search(seat, library(&engine, seat, 3));

    let view = player_view(
        engine.state(),
        seat,
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    for object in &view.looking_at {
        let print = object
            .card
            .expect("a library card is known to its owner")
            .print;
        assert!(
            view.prints().any(|p| p == print),
            "a card being shown did not earn its printing"
        );
    }
}
/// A seat's own hand is in its view already, so being asked about it
/// shows nothing twice. This is the arm of [`shown_elsewhere`] with a
/// judgement in it: hand is the one zone whose visibility depends on
/// whose hand it is.
#[test]
fn a_seat_asked_about_its_own_hand_is_shown_nothing_extra() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let seat = PlayerId::new(0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(seat)).clone();
    assert!(!hand.is_empty(), "the preset deals a starting hand");
    let pending = Pending::ChooseCards {
        player: seat,
        options: hand,
        min: 1,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::PutBackOnTop,
        total: None,
    };

    let view = player_view(
        engine.state(),
        seat,
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    assert!(
        view.looking_at.is_empty(),
        "a seat's own hand was repeated as something it is being shown"
    );
}

/// And the other side of the same arm, which is where the whole rule is
/// worth its cost: a discard-at-random or a Thoughtseize asks one seat
/// about *another* seat's hand. Those cards are hidden from everyone by
/// default, and the question is what entitles this seat to them — so the
/// asked seat sees them, in full, and only while it is asked.
#[test]
fn a_seat_asked_about_another_hand_is_shown_it() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
    assert!(!hand.is_empty(), "the preset deals a starting hand");
    let pending = Pending::ChooseCards {
        player: me,
        options: hand.clone(),
        min: 1,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::Generic,
        total: None,
    };

    let view = player_view(
        engine.state(),
        me,
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    let shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
    assert_eq!(
        shown, hand,
        "the asked seat was not shown the hand in question"
    );
    assert!(
        view.looking_at.iter().all(|o| o.card.is_some()),
        "a card this seat is being asked about arrived without its identity"
    );

    // The owner of that hand is being asked nothing, and is shown nothing.
    let theirs = player_view(
        engine.state(),
        them,
        0,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    assert!(
        theirs.looking_at.is_empty(),
        "a seat not being asked was handed a list anyway"
    );
}
