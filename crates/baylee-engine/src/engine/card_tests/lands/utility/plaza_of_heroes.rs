//! `cards/lands/utility/plaza_of_heroes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plaza of Heroes: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a legendary spell." / "{T}: Add one mana of any color among legendary permanents you control."
/// Under `Coverage::Partial`, the mana ability reading colors among controlled legendary permanents is omitted.
/// Activating ability 1 produces one restricted mana of any chosen color in `pool.restricted()` rather than general available mana.
#[test]
fn plaza_of_heroes_produces_restricted_legendary_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(339, forest())
        .battlefield(0, &[plaza_of_heroes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, plaza_of_heroes(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert_eq!(pool.available(ManaColor::White), 0);
    let land = on_battlefield(&engine, p0, plaza_of_heroes()).expect("land on battlefield");
    assert!(is_tapped(&engine, land));
}
