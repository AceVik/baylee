//! `cards/lands/secluded_courtyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Secluded Courtyard prints `As this land enters, choose a creature type.`,
/// `{{T}}: Add {{C}}.`, and `{{T}}: Add one mana of any color. Spend this mana only
/// to cast a creature spell of the chosen type or activate an ability of a creature
/// source of the chosen type.`
///
/// Under `Coverage::Partial`, the any-color mana pays for creature spells of the
/// chosen type only: its activated-ability half has no shape, and restricted mana
/// pays for no activation. Playing the land prompts for a creature subtype
/// via `Pending::ChooseSubtype`. Activating ability 1 prompts for a color choice
/// via `Pending::ChooseColor` and produces one restricted mana in `pool.restricted()`
/// rather than `pool.available()`.
#[test]
fn secluded_courtyard_chooses_subtype_and_produces_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[secluded_courtyard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, secluded_courtyard()).expect("courtyard in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected ChooseSubtype, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    engine
        .apply(p0, PlayerAction::ChooseSubtype(options[0]))
        .unwrap();

    let land = on_battlefield(&engine, p0, secluded_courtyard()).expect("courtyard on battlefield");
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, secluded_courtyard(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
