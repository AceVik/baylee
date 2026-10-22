//! `cards/creatures/mv_1/roc_hatchling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ROC_HATCHLING;

/// `Roc Hatchling` ({R}, Creature — Bird 0/1) enters with four shell counters,
/// removes one at your upkeep, and gets +3/+2 and flying as long as it has no
/// shell counters on it.
#[test]
fn roc_hatchling_enters_with_shell_counters_and_grows_when_empty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[ROC_HATCHLING])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ROC_HATCHLING);
    pass_until(&mut engine, stack_is_empty);

    let roc = on_battlefield(&engine, p0, ROC_HATCHLING).unwrap();
    assert_eq!(
        engine
            .state()
            .object(roc)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::SHELL),
        4
    );
    assert_eq!(pt(&engine, roc), (0, 1));
    assert!(!keywords(&engine, roc).contains(KeywordSet::FLYING));

    engine
        .state
        .object_mut(roc)
        .unwrap()
        .counters
        .set(baylee_cards_dsl::counters::SHELL, 1);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Main
    });

    assert_eq!(
        engine
            .state()
            .object(roc)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::SHELL),
        0
    );
    assert_eq!(pt(&engine, roc), (3, 3));
    assert!(keywords(&engine, roc).contains(KeywordSet::FLYING));
}
