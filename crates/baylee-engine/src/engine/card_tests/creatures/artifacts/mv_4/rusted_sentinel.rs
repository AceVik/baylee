//! `cards/creatures/artifacts/mv_4/rusted_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rusted_sentinel() -> CardIndex {
    card_index("bab28608-b388-4c48-92a4-689943bbbf4a")
}

/// Rusted Sentinel — {4} 3/4 artifact creature: "This creature enters
/// tapped." Cast off four Forests it comes in tapped.
#[test]
fn rusted_sentinel_enters_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[rusted_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, rusted_sentinel());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, rusted_sentinel()).expect("it resolved");
    assert_eq!(pt(&engine, golem), (3, 4));
    assert!(is_tapped(&engine, golem), "it entered tapped");
}
