//! `cards/creatures/artifacts/mv_4/gilded_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gilded_sentinel() -> CardIndex {
    card_index("a382165d-d715-4836-b1ce-9e5d8dc56fb4")
}

/// Gilded Sentinel — {4} 3/3 artifact creature with no rules text. Cast off
/// four Forests it resolves untapped as a vanilla 3/3: no keyword, no
/// ability offered.
#[test]
fn gilded_sentinel_is_a_plain_three_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[gilded_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, gilded_sentinel());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, gilded_sentinel()).expect("it resolved");
    assert_eq!(pt(&engine, golem), (3, 3));
    assert!(!is_tapped(&engine, golem), "it enters untapped");
    assert_eq!(keywords(&engine, golem), KeywordSet::EMPTY);
    assert!(
        priority_offer(&engine)
            .abilities
            .iter()
            .all(|(source, _)| *source != golem),
        "no ability of its own is on offer"
    );
}
