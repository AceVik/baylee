//! `cards/creatures/artifacts/mv_5/teferi_s_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn teferi_s_sentinel() -> CardIndex {
    card_index("bf24c063-dd9a-4381-b94e-682b0cb0c03d")
}

fn teferi_time_raveler() -> CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
}

/// Teferi's Sentinel — "As long as you control a Teferi planeswalker, this
/// creature gets +4/+0." A Teferi arrives from hand and the Golem is 6/6; a
/// different planeswalker leaves it a 2/6.
#[test]
fn teferis_sentinel_grows_beside_a_teferi_planeswalker() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[teferi_time_raveler()])
        .battlefield(0, &[teferi_s_sentinel(), plains(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sentinel = on_battlefield(&engine, p0, teferi_s_sentinel()).expect("seated");
    assert_eq!(pt(&engine, sentinel), (2, 6));
    cast_from_hand(&mut engine, p0, teferi_time_raveler());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, teferi_time_raveler()).is_some());
    assert_eq!(pt(&engine, sentinel), (6, 6), "+4/+0");
}

#[test]
fn teferis_sentinel_ignores_other_planeswalkers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[liliana_the_repentant()])
        .battlefield(0, &[teferi_s_sentinel(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sentinel = on_battlefield(&engine, p0, teferi_s_sentinel()).expect("seated");
    cast_from_hand(&mut engine, p0, liliana_the_repentant());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, liliana_the_repentant()).is_some());
    assert_eq!(pt(&engine, sentinel), (2, 6), "Liliana is not a Teferi");
}
