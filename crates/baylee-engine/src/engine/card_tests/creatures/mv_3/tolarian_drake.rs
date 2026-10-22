//! `cards/creatures/mv_3/tolarian_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn tolarian_drake() -> CardIndex {
    card_index("d999f2a3-5cb4-4f12-a79a-c5db0a300620")
}

/// `Tolarian Drake` ({2}{U}, 2/4 Drake) has flying and phasing, and phases out
/// during its controller's next untap step.
#[test]
fn tolarian_drake_has_flying_and_phases_out_at_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[tolarian_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tolarian_drake());
    pass_until(&mut engine, stack_is_empty);

    let drake = on_battlefield(&engine, p0, tolarian_drake()).expect("Tolarian Drake");
    assert_eq!(pt(&engine, drake), (2, 4));
    assert!(
        engine
            .state()
            .object(drake)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING.union(KeywordSet::PHASING))
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        engine
            .state()
            .object(drake)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "Tolarian Drake phased out during its controller's untap step"
    );
}
