//! `cards/creatures/artifacts/mv_2/manakin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Manakin` prints `{{T}}: Add {{C}}` on a 1/1 artifact creature construct with `Coverage::Implemented`.
/// Seated on the battlefield from turn one, it stands untapped, unsick, and with no other mana sources
/// present. When `tap_all_mana` is called, the construct taps for mana directly without using the stack,
/// providing exactly one colorless mana into seat 0's mana pool.
#[test]
fn manakin_taps_for_one_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[manakin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let construct = on_battlefield(&engine, p0, manakin()).expect("manakin is seated");
    assert_eq!(pt(&engine, construct), (1, 1));
    let t = types(&engine, construct);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(!is_tapped(&engine, construct));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana(&mut engine, p0);

    assert!(is_tapped(&engine, construct));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(stack_is_empty(&engine));
}
