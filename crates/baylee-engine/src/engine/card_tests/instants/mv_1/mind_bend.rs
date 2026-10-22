//! `cards/instants/mv_1/mind_bend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "d8377869-f8f7-4b4b-93ad-3ad3c4826b1a"
fn mind_bend() -> CardIndex {
    card_index("d8377869-f8f7-4b4b-93ad-3ad3c4826b1a")
}

#[test]
fn mind_bend_changes_color_word_on_target_permanent_indefinitely() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                forest(),
                index::BLACK_KNIGHT,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[mind_bend(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let knight = on_battlefield(&engine, p0, index::BLACK_KNIGHT).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, mind_bend());
    aim_at(&mut engine, p0, knight);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Mode 0: replace a color word with another.
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // WUBRG order: white (0) -> green (4) is 0 * 4 + (4 - 1) = 3.
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending())
    };
    assert!(
        !options.contains(&knight),
        "Black Knight now has protection from green"
    );
    assert!(options.contains(&bears));
}
