//! `cards/lands/tapland/primal_beyond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Primal Beyond: "As this land enters, you may reveal an Elemental card from your hand..." / "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast an Elemental spell..."
/// Under `Coverage::Partial`, the reveal-from-hand enter clause is omitted, and restricted mana applies to spells only.
/// Activating ability 1 produces one restricted mana of any chosen color in `pool.restricted()` rather than general available mana.
#[test]
fn primal_beyond_produces_restricted_elemental_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(328, forest())
        .battlefield(0, &[primal_beyond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, primal_beyond(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert_eq!(pool.available(ManaColor::Red), 0);
    let land = on_battlefield(&engine, p0, primal_beyond()).expect("land on battlefield");
    assert!(is_tapped(&engine, land));
}
