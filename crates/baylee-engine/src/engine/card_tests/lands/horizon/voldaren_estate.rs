//! `cards/lands/horizon/voldaren_estate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The unrestricted mana ability pays life and produces a chosen color.
#[test]
fn voldaren_estate_pays_life_for_restricted_vampire_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(212, forest())
        .battlefield(0, &[voldaren_estate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let estate = on_battlefield(&engine, p0, voldaren_estate()).expect("Estate deployed");
    activate(&mut engine, p0, voldaren_estate(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(engine.state().players[0].life, 19);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Black);
    assert!(is_tapped(&engine, estate));
}

#[test]
fn voldaren_estate_counts_your_vampires_and_creates_blood() {
    use crate::engine::card_tests::legends_kit as kit;
    let vampire = card_index("310f141c-7f37-4729-aed6-dd9c09db448d");
    let mut engine = kit::game(
        938,
        &[voldaren_estate(), vampire, vampire, llanowar_elves()],
        &[vampire],
        &[],
        &[],
    );
    kit::float(&mut engine, kit::P0, &[(ManaColor::Colorless, 2)]);
    assert!(!kit::offered(&engine, voldaren_estate(), kit::P0, 2));
    kit::float(&mut engine, kit::P0, &[(ManaColor::Colorless, 1)]);
    kit::use_ability(&mut engine, kit::P0, voldaren_estate(), 2, &[], &[]);
    assert_eq!(kit::pool_total(&engine, kit::P0), 0);
    let token = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .last()
        .unwrap();
    assert!(
        engine
            .state()
            .object(token)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::BLOOD)
    );
}
