//! `cards/sorceries/mv_3/spirit_water_revival.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Declining the waterbend declines its taps (CR 701.67b): after "waterbend
/// {6}?" is answered no, the cast asks nothing more and taps nothing.
#[test]
fn spirit_water_revival_asks_for_no_taps_when_the_waterbend_is_declined() {
    let seat = PlayerId::new(0);
    let mut engine = revival_table(&[
        island(),
        island(),
        island(),
        ondu_cleric(),
        darksteel_pendant(),
    ]);
    let revival = in_hand(&engine, seat, spirit_water_revival()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: revival })
        .expect("three Islands pay {1}{U}{U}");
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Kicker,
                ..
            }
        ),
        "the waterbend was not asked: {:?}",
        engine.pending()
    );
    engine
        .apply(seat, PlayerAction::YesNo(false))
        .expect("the waterbend is optional");
    assert_eq!(
        tap_to_pay_question(&engine),
        None,
        "a declined waterbend still asked for taps"
    );
    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell never reached the stack"
    );
    for card in [ondu_cleric(), darksteel_pendant()] {
        let id = on_battlefield(&engine, seat, card).expect("on the battlefield");
        assert!(
            !is_tapped(&engine, id),
            "a declined waterbend tapped something"
        );
    }
}

/// A paid waterbend taps artifacts as well as creatures (CR 701.67a), and
/// only as many as its `{6}` has generic mana (CR 701.67b): eight bodies are
/// offered, six may be tapped, and the Islands pay the printed cost.
#[test]
fn spirit_water_revival_s_waterbend_taps_pay_the_six_and_no_more() {
    let seat = PlayerId::new(0);
    let mut board = vec![island(), island(), island(), darksteel_pendant()];
    board.extend(std::iter::repeat_n(ondu_cleric(), 7));
    let mut engine = revival_table(&board);
    let revival = in_hand(&engine, seat, spirit_water_revival()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: revival })
        .expect("three Islands pay {1}{U}{U}");
    engine
        .apply(seat, PlayerAction::YesNo(true))
        .expect("the waterbend is taken");
    let (options, max) = tap_to_pay_question(&engine).expect("the waterbend asks for its taps");
    let pendant = on_battlefield(&engine, seat, darksteel_pendant()).expect("on the battlefield");
    assert_eq!(options.len(), 8, "seven creatures and an artifact may help");
    assert!(options.contains(&pendant), "waterbend may tap an artifact");
    assert_eq!(
        max, 6,
        "the taps pay the waterbend's {{6}} and nothing past it"
    );
    let tapped: Vec<ObjectId> = options.iter().copied().take(6).collect();
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: tapped.clone(),
                players: vec![],
            },
        )
        .expect("six taps and three Islands pay {7}{U}{U}");
    assert!(
        !engine.state().zones.stack_is_empty(),
        "the spell never reached the stack"
    );
    assert!(
        tapped.iter().all(|id| is_tapped(&engine, *id)),
        "a tap was not spent"
    );
}

// ---- Abilities no test had fired (L4 sweep, 2026-10-01) ----

/// Spirit Water Revival, waterbend declined: "Draw two cards." and "Exile
/// Spirit Water Revival." Nothing else tested the spell once it resolves.
#[test]
fn spirit_water_revival_draws_two_and_exiles_itself_when_the_waterbend_is_declined() {
    let seat = PlayerId::new(0);
    let mut engine = revival_table(&[island(), island(), island()]);
    let revival = in_hand(&engine, seat, spirit_water_revival()).expect("in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(seat)).len();
    engine
        .apply(seat, PlayerAction::CastSpell { card: revival })
        .unwrap();
    engine.apply(seat, PlayerAction::YesNo(false)).unwrap();
    let lib = library_size(&engine, seat);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(seat)).len(),
        hand_before - 1 + 2,
        "the spell left the hand and two cards came in"
    );
    assert_eq!(library_size(&engine, seat), lib - 2);
    assert_eq!(exiled(&engine, seat, spirit_water_revival()), 1);
    assert!(in_graveyard(&engine, seat, spirit_water_revival()).is_none());
}

/// Spirit Water Revival, waterbend paid: "instead shuffle your graveyard
/// into your library, draw seven cards". The graveyard is seeded with three
/// cards; afterwards it is empty bar nothing (the spell is exiled), the
/// library gained three and lost seven, and the hand holds seven.
#[test]
fn spirit_water_revival_paid_shuffles_the_graveyard_in_and_draws_seven() {
    let seat = PlayerId::new(0);
    let mut board = vec![island(), island(), island()];
    board.extend(std::iter::repeat_n(ondu_cleric(), 6));
    let mut engine = revival_table(&board);
    seed_graveyard(&mut engine, seat, 3);
    let revival = in_hand(&engine, seat, spirit_water_revival()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: revival })
        .unwrap();
    engine.apply(seat, PlayerAction::YesNo(true)).unwrap();
    let (options, max) = tap_to_pay_question(&engine).expect("the waterbend asks for taps");
    assert_eq!(max, 6);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: options.into_iter().take(6).collect(),
                players: vec![],
            },
        )
        .unwrap();
    let lib = library_size(&engine, seat);
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .len();
    assert_eq!(gy, 3, "the seeded graveyard");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(seat)).len(),
        7,
        "seven cards drawn into an empty hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(seat))
            .is_empty(),
        "the graveyard was shuffled away and the spell exiled"
    );
    assert_eq!(library_size(&engine, seat), lib + 3 - 7);
    assert_eq!(exiled(&engine, seat, spirit_water_revival()), 1);
}
