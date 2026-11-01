//! `cards/creatures/mv_3/skyshroud_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn skyshroud_sentinel_searches_up_to_three_copies_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, card_index("4f437f42-1ac5-4681-9b16-4eae9f49bb22"))
        .hand(0, &[card_index("4f437f42-1ac5-4681-9b16-4eae9f49bb22")])
        .battlefield(0, &[forest(); 3])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("4f437f42-1ac5-4681-9b16-4eae9f49bb22"),
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards for Skyshroud Sentinel");
    };
    assert_eq!(min, 0);
    assert_eq!(max, 3);
    let chosen = vec![options[0], options[1]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for id in chosen {
        assert_eq!(engine.state().object(id).unwrap().zone, Zone::Hand);
    }
}
