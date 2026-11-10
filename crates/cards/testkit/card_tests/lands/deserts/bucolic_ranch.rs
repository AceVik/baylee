//! `cards/lands/deserts/bucolic_ranch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bucolic Ranch: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Mount spell." / "{3}, {T}: Look at the top card of your library..."
/// Under `Coverage::Partial`, the top-card inspection ability is omitted because no effect offers a conditional keep-or-bottom.
/// Activating ability 1 produces one restricted mana of any chosen color in `pool.restricted()` rather than general available mana.
#[test]
fn bucolic_ranch_adds_restricted_mount_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(209, forest())
        .battlefield(0, &[bucolic_ranch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ranch = on_battlefield(&engine, p0, bucolic_ranch()).expect("ranch deployed");
    activate(&mut engine, p0, bucolic_ranch(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::White);
    assert!(is_tapped(&engine, ranch));
}
