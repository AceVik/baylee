//! `cards/creatures/artifacts/mv_2/iron_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Iron Myr` prints `{{T}}: Add {{R}}` on a 1/1 artifact creature with `Coverage::Implemented`.
/// Seated on the battlefield from turn one, it stands untapped, unsick, and with no other mana sources
/// present. When `tap_all_mana` is called, the Myr taps for mana directly without using the stack,
/// providing exactly one red mana into seat 0's mana pool.
#[test]
fn iron_myr_taps_for_one_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[iron_myr()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let myr = on_battlefield(&engine, p0, iron_myr()).expect("iron myr is seated");
    assert_eq!(pt(&engine, myr), (1, 1));
    let t = types(&engine, myr);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(!is_tapped(&engine, myr));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana(&mut engine, p0);

    assert!(is_tapped(&engine, myr));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.total(), 1);
    assert!(stack_is_empty(&engine));
}
