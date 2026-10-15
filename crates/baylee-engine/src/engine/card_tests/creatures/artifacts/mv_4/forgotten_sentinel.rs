//! `cards/creatures/artifacts/mv_4/forgotten_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn forgotten_sentinel() -> CardIndex {
    card_index("9bd4a0bd-d10c-434a-8254-89f38740aafd")
}

/// Forgotten Sentinel — {4} 4/3 artifact creature: "This creature enters
/// tapped." Cast off four Forests it comes in tapped; a seeded board is a
/// placement and not an entry, so the card has to be cast to be read.
#[test]
fn forgotten_sentinel_enters_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[forgotten_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, forgotten_sentinel());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, forgotten_sentinel()).expect("it resolved");
    assert_eq!(pt(&engine, golem), (4, 3));
    assert!(is_tapped(&engine, golem), "it entered tapped");
}
