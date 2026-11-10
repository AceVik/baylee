//! `cards/lands/restricted/gleaming_bastion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gleaming Bastion: "{T}: Add {C}." / "{T}: Add {W} or {U}. Activate only if this land entered this turn or if you control a basic land."
/// Under `Coverage::Partial`, the colored ability is implemented with the basic-land control condition.
/// Controlling a basic Plains enables ability 1, offering White or Blue and adding {U} to the mana pool.
#[test]
fn gleaming_bastion_with_basic_land_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(109, plains())
        .battlefield(0, &[gleaming_bastion(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bastion = on_battlefield(&engine, p0, gleaming_bastion()).expect("Bastion deployed");
    activate(&mut engine, p0, gleaming_bastion(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, bastion));
}

/// Gleaming Bastion prints `{{T}}: Add {{C}}.` and
/// `{{T}}: Add {{W}} or {{U}}. Activate only if this land entered this turn or if you control a basic land.`
/// Under `Coverage::Implemented`, both branches of the disjunction are supported.
/// This test plays Gleaming Bastion with no basic land on the battlefield, activates ability 1 for blue mana
/// on the turn it entered via the entry clause, passes to the next turn where ability 1 is withheld without
/// a basic land, plays a basic `plains()`, and confirms ability 1 is now offered and produces white mana.
#[test]
fn gleaming_bastion_activates_on_entry_turn_or_with_basic_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[gleaming_bastion(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bastion = play_land(&mut engine, p0, gleaming_bastion());
    assert!(
        !entered_tapped(&engine, bastion),
        "Gleaming Bastion enters untapped"
    );

    // On the entry turn, ability 1 is legal via the entered-this-turn clause even with no basic land.
    activate(&mut engine, p0, gleaming_bastion(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

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
    assert!(is_tapped(&engine, bastion), "Gleaming Bastion is tapped");

    // On the following turn, Gleaming Bastion untaps. Without a basic land and having not
    // entered this turn, ability 1 is withheld while ability 0 remains available.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, bastion), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == bastion && *idx == 1),
        "ability 1 is withheld without basic land and not having entered this turn"
    );
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == bastion && *idx == 0),
        "ability 0 is offered"
    );

    // Play a basic Plains to satisfy the second branch of the disjunction.
    let _land = play_land(&mut engine, p0, plains());

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == bastion && *idx == 1),
        "ability 1 is offered now that a basic land is controlled"
    );

    activate(&mut engine, p0, gleaming_bastion(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));

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
    assert!(is_tapped(&engine, bastion));
}
