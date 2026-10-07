//! `cards/lands/utility/villainous_hideout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Villainous Hideout: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Villain spell..." / "{3}, {T}: Target Villain you control connives."
/// Under `Coverage::Partial`, the connive ability is omitted because connive has no effect representation.
/// Activating ability 1 adds one restricted mana of any chosen color to `pool.restricted()`.
#[test]
fn villainous_hideout_adds_restricted_villain_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(231, forest())
        .battlefield(0, &[villainous_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hideout = on_battlefield(&engine, p0, villainous_hideout()).expect("Hideout deployed");
    activate(&mut engine, p0, villainous_hideout(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert!(is_tapped(&engine, hideout));
}
