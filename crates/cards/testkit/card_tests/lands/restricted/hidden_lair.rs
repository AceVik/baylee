//! `cards/lands/restricted/hidden_lair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hidden Lair: "{T}: Add {C}." / "{T}: Add {U} or {B}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the colored ability is implemented with the basic-land control condition.
/// Controlling a basic Swamp enables ability 1, offering Blue or Black and adding {U} to the mana pool.
#[test]
fn hidden_lair_with_basic_land_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, swamp())
        .battlefield(0, &[hidden_lair(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = on_battlefield(&engine, p0, hidden_lair()).expect("Hidden Lair deployed");
    activate(&mut engine, p0, hidden_lair(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, lair));
}

/// Hidden Lair prints `{{T}}: Add {{C}}.` and
/// `{{T}}: Add {{U}} or {{B}}. Activate only if this land entered this turn or if you control a basic land.`
/// Under `Coverage::Implemented`, both branches of the disjunction are supported.
/// This test plays Hidden Lair with no basic land on the battlefield, activates ability 1 for black mana
/// on the turn it entered via the entry clause, passes to the next turn where ability 1 is withheld without
/// a basic land, plays a basic `swamp()`, and confirms ability 1 is now offered and produces blue mana.
#[test]
fn hidden_lair_activates_on_entry_turn_or_with_basic_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[hidden_lair(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lair = play_land(&mut engine, p0, hidden_lair());
    assert!(
        !entered_tapped(&engine, lair),
        "Hidden Lair enters untapped"
    );

    // On the entry turn, ability 1 is legal via the entered-this-turn clause even with no basic land.
    activate(&mut engine, p0, hidden_lair(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("choosing black mana is legal");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "adds one black mana"
    );
    assert!(is_tapped(&engine, lair), "Hidden Lair is tapped");

    // On the following turn, Hidden Lair untaps. Without a basic land and having not
    // entered this turn, ability 1 is withheld while ability 0 remains available.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, lair), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == lair && *idx == 1),
        "ability 1 is withheld without basic land and not having entered this turn"
    );
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == lair && *idx == 0),
        "ability 0 is offered"
    );

    // Play a basic Swamp to satisfy the second branch of the disjunction.
    let _land = play_land(&mut engine, p0, swamp());

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == lair && *idx == 1),
        "ability 1 is offered now that a basic land is controlled"
    );

    activate(&mut engine, p0, hidden_lair(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choosing blue mana is legal");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "adds one blue mana"
    );
    assert!(is_tapped(&engine, lair));
}
