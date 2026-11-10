//! `cards/lands/restricted/gathering_place.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gathering Place: "{T}: Add {C}." / "{T}: Add {G} or {W}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the colored ability is implemented with the basic-land condition.
/// Controlling a basic Forest enables ability 1, prompting for a color choice and adding {W} to the pool.
#[test]
fn gathering_place_with_basic_land_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, forest())
        .battlefield(0, &[gathering_place(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gp = on_battlefield(&engine, p0, gathering_place()).expect("Gathering Place deployed");
    activate(&mut engine, p0, gathering_place(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, gp));
}

/// Gathering Place prints `{{T}}: Add {{C}}.` and
/// `{{T}}: Add {{G}} or {{W}}. Activate only if this land entered this turn or if you control a basic land.`
/// Under `Coverage::Implemented`, both branches of the disjunction are supported.
/// This test plays Gathering Place with no basic land on the battlefield, activates ability 1 for white mana
/// on the turn it entered via the entry clause, passes to the next turn where ability 1 is withheld without
/// a basic land, plays a basic `forest()`, and confirms ability 1 is now offered and produces green mana.
#[test]
fn gathering_place_activates_on_entry_turn_or_with_basic_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gathering_place(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gp = play_land(&mut engine, p0, gathering_place());
    assert!(
        !entered_tapped(&engine, gp),
        "Gathering Place enters untapped"
    );

    // On the entry turn, ability 1 is legal via the entered-this-turn clause even with no basic land.
    activate(&mut engine, p0, gathering_place(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("choosing white mana is legal");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "adds one white mana"
    );
    assert!(is_tapped(&engine, gp), "Gathering Place is tapped");

    // On the following turn, Gathering Place untaps. Without a basic land and having not
    // entered this turn, ability 1 is withheld while ability 0 remains available.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, gp), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == gp && *idx == 1),
        "ability 1 is withheld without basic land and not having entered this turn"
    );
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == gp && *idx == 0),
        "ability 0 is offered"
    );

    // Play a basic Forest to satisfy the second branch of the disjunction.
    let _land = play_land(&mut engine, p0, forest());

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == gp && *idx == 1),
        "ability 1 is offered now that a basic land is controlled"
    );

    activate(&mut engine, p0, gathering_place(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("choosing green mana is legal");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "adds one green mana"
    );
    assert!(is_tapped(&engine, gp));
}
