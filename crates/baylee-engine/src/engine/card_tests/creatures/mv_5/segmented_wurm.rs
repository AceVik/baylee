//! `cards/creatures/mv_5/segmented_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::TWIDDLE;

fn segmented_wurm() -> CardIndex {
    card_index("eb1baf72-4ef6-4967-8a22-2022c633f3d3")
}

/// `Segmented Wurm` ({3}{R}{G}, 5/5 Creature — Wurm, `Coverage::Implemented`):
/// `Whenever this creature becomes the target of a spell or ability, put a
/// -1/-1 counter on it.`
#[test]
fn segmented_wurm_gets_a_minus_one_minus_one_counter_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), segmented_wurm()])
        .hand(0, &[TWIDDLE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, segmented_wurm()).unwrap();
    let ch = engine.state().object(wurm).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(5), Some(5)));

    cast_from_hand(&mut engine, p0, TWIDDLE);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wurm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ch = engine.state().object(wurm).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(4), Some(4)));
}
