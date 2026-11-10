//! `cards/creatures/artifacts/mv_2/leaden_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Leaden Myr` prints `{{T}}: Add {{B}}` on a 1/1 artifact creature with `Coverage::Implemented`.
/// Seated on the battlefield from turn one, it stands untapped, unsick, and with no other mana sources
/// present. When `tap_all_mana` is called, the Myr taps for mana directly without using the stack,
/// providing exactly one black mana into seat 0's mana pool.
#[test]
fn leaden_myr_taps_for_one_black_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[leaden_myr()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let myr = on_battlefield(&engine, p0, leaden_myr()).expect("leaden myr is seated");
    assert_eq!(pt(&engine, myr), (1, 1));
    let t = types(&engine, myr);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(!is_tapped(&engine, myr));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana(&mut engine, p0);

    assert!(is_tapped(&engine, myr));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 1);
    assert!(stack_is_empty(&engine));
}
