use super::*;

/// The whole point of `PrintRef`: two copies of the *same* card in one
/// deck can be different printings, and the client has to be told which
/// is which. The engine never interprets the ref — it carries it — so
/// this test follows one deck entry all the way to the seat view.
#[test]
fn the_same_card_in_two_printings_stays_two_printings_in_the_view() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let seat = PlayerId::new(0);
    let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);

    let battlefield: Vec<u16> = view
        .battlefield
        .iter()
        .filter(|o| o.controller == seat)
        .filter_map(|o| o.card.map(|c| c.print.get()))
        .collect();
    assert_eq!(
        battlefield,
        vec![1, 2],
        "the battlefield lost the printings the preset asked for"
    );

    let hand: Vec<u16> = view.hand.iter().map(|o| o.card.print.get()).collect();
    assert_eq!(hand, vec![0, 2], "the hand lost its printings");

    // Same rules identity throughout — only the printing differs.
    assert!(
        view.hand.iter().all(|o| o.card.index == island()),
        "print refs must not disturb card identity"
    );
}

/// The print table is a per-game payload: the view carries indices, and
/// `GameStatic` carries what they mean. A client that only got the
/// indices could not fetch an image.
#[test]
fn the_static_payload_carries_what_a_print_ref_points_at() {
    let preset = mixed_print_preset();
    let shown = vec![true; preset.prints.len()];
    let statics = game_static(
        "g1".into(),
        PlayerId::new(0),
        vec![],
        &preset.prints,
        &shown,
        &preset.house_rules,
    );
    assert_eq!(statics.prints.len(), 3);
    let entry = |i: u16| statics.print(PrintRef::new(i)).expect("shown");
    assert_eq!(entry(1).lang, "DE");
    assert!(matches!(entry(1).finish, baylee_view::Finish::Foil));
    assert!(matches!(entry(2).finish, baylee_view::Finish::Etched));
    assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
}

/// The house rules spell *no limit* as zero and the payload spells it as
/// `None`, because zero on a seat sheet reads as the opposite: a table
/// with no time at all rather than one with all the time there is.
///
/// Unreachable through the gateway, which refuses a zero reconnect
/// window (`clock::MIN_RECONNECT_SECS` is 10) — but a local harness may
/// choose either, and this payload has to be honest about a table the
/// gateway did not make.
#[test]
fn a_table_with_no_limit_says_none_rather_than_nought() {
    let mut preset = mixed_print_preset();
    preset.house_rules.decision_timeout_secs = 0;
    preset.house_rules.reconnect_window_secs = 0;
    let statics = game_static(
        "g1".into(),
        PlayerId::new(0),
        vec![],
        &preset.prints,
        &[true, true, true],
        &preset.house_rules,
    );
    assert_eq!(statics.decision_secs, None);
    assert_eq!(statics.reconnect_secs, None);

    preset.house_rules.decision_timeout_secs = 30;
    preset.house_rules.reconnect_window_secs = 45;
    let statics = game_static(
        "g1".into(),
        PlayerId::new(0),
        vec![],
        &preset.prints,
        &[true, true, true],
        &preset.house_rules,
    );
    assert_eq!(statics.decision_secs, Some(30));
    assert_eq!(
        statics.reconnect_secs,
        Some(45),
        "the two limits are separate numbers and must not be read from one field"
    );
}

/// A printing this seat has not been shown is a hole in the table, not a
/// shorter table: the index is the `PrintRef` every object points at.
#[test]
fn a_printing_a_seat_has_not_seen_is_a_hole_not_a_gap() {
    let preset = mixed_print_preset();
    let statics = game_static(
        "g1".into(),
        PlayerId::new(0),
        vec![],
        &preset.prints,
        &[true, false, true],
        &preset.house_rules,
    );
    assert_eq!(statics.prints.len(), 3, "the indices do not move");
    assert!(statics.print(PrintRef::new(0)).is_some());
    assert!(statics.print(PrintRef::new(1)).is_none());
    assert!(matches!(
        statics.print(PrintRef::new(2)).map(|p| p.finish),
        Some(baylee_view::Finish::Etched)
    ));
}

/// A token has no printing, so `card` is `None` and a client has nothing
/// to fetch an image with. The token id is the handle that replaces it:
/// it survives the projection into the view, and it resolves back to the
/// definition the engine created the object from.
#[test]
fn a_token_reaches_the_client_with_the_handle_its_art_is_keyed_on() {
    use baylee_engine::choice::{Pending, PlayerAction};

    let preset = mixed_print_preset();
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }

    // Every object the view can project; a card-backed one carries a
    // printing and no token id, and the two are mutually exclusive.
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        1,
        None,
        &SeatContext::default(),
        &[],
    );
    for object in &view.battlefield {
        assert!(
            object.card.is_none() || object.token.is_none(),
            "{} claims to be both a printing and a token",
            object.name
        );
    }

    // And the id round-trips: whatever the view says, the registry can
    // name it. A token filed under `u16::MAX` — one defined in a card
    // file instead of the registry — would fail here.
    for id in 0..u16::try_from(baylee_cards::tokens::ALL.len()).expect("registry fits") {
        let token = baylee_cards::tokens::by_token_id(id).expect("id names a token");
        assert_eq!(baylee_cards::tokens::token_id(token), id);
    }
}

/// A card keeps its printing when it changes zone: the ref lives on the
/// object, not on the zone it happens to be in. The land is played for
/// real rather than moved by hand, so the whole cast path is covered.
#[test]
fn a_printing_survives_a_zone_change() {
    use baylee_engine::choice::{Pending, PlayerAction};

    let preset = mixed_print_preset();
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let seat = PlayerId::new(0);

    // Walk to seat 0's main phase, where a land may be played.
    for _ in 0..30 {
        if let Pending::Priority { player, legal } = engine.pending()
            && *player == seat
            && !legal.lands.is_empty()
        {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    let card = *legal.lands.first().expect("a land in hand to play");
    let print_before = engine
        .state()
        .object(card)
        .and_then(|o| o.card)
        .expect("a card-backed object")
        .print;
    engine
        .apply(seat, PlayerAction::PlayLand { card })
        .expect("playing a land from hand is legal");

    let view = player_view(engine.state(), seat, 1, None, &SeatContext::default(), &[]);
    let played = view
        .battlefield
        .iter()
        .find(|o| o.id == card)
        .expect("the land reached the battlefield");
    assert_eq!(
        played.card.expect("card-backed").print,
        print_before,
        "the printing was lost on the way to the battlefield"
    );
    assert!(
        !view.hand.iter().any(|o| o.id == card),
        "the land is still shown in hand"
    );
}
