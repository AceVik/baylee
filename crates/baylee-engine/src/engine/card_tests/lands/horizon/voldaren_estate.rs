//! `cards/lands/horizon/voldaren_estate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Voldaren Estate: "{T}: Add {C}." / "{T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a Vampire spell." / "{5}, {T}: Create a Blood token..."
/// Under `Coverage::Partial`, the Blood token ability is omitted because the Vampire cost reduction is inexpressible.
/// Activating ability 1 pays 1 life and adds one restricted mana to `pool.restricted()`.
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
