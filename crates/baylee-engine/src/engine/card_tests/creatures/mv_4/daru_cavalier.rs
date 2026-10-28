//! `cards/creatures/mv_4/daru_cavalier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn daru_cavalier_etb_searches_library_for_another_copy_to_hand() {
    let p0 = PlayerId::new(0);
    let cavalier = card_index("ba0291ad-09ea-4fee-baa9-7778fbbb6ba7");
    let mut engine = Duel::new(SEED, cavalier)
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[cavalier])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = hand_size(&engine, p0);
    cast_from_hand(&mut engine, p0, cavalier);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let cavalier_id = on_battlefield(&engine, p0, cavalier).unwrap();
    assert!(keywords(&engine, cavalier_id).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(hand_size(&engine, p0), hand_before);
}
