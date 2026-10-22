//! `cards/instants/mv_1/whim_of_volrath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn whim_of_volrath() -> CardIndex {
    card_index("51a20c9f-68d2-471b-8622-67d481b53b92")
}

/// `Whim of Volrath` ({U}, Instant, `Coverage::Partial`):
/// `Buyback {2}`
/// `Change the text of target permanent by replacing all instances of one color
/// word with another or one basic land type with another until end of turn.`
#[test]
fn whim_of_volrath_changes_color_word_on_target_permanent() {
    let p0 = PlayerId::new(0);
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
        .hand(0, &[whim_of_volrath(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let knight = on_battlefield(&engine, p0, index::BLACK_KNIGHT).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, whim_of_volrath());
    aim_at(&mut engine, p0, knight);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
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
