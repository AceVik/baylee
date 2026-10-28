//! `cards/creatures/mv_5/avarax.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn avarax_searches_for_another_copy_on_etb_and_pumps_itself() {
    let p0 = PlayerId::new(0);
    let avarax = card_index("2e1deb7e-d40d-437e-b817-b66132d2d1d6");
    let mut engine = Duel::new(SEED, avarax)
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[avarax])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = hand_size(&engine, p0);
    cast_from_hand(&mut engine, p0, avarax);
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

    let beast = on_battlefield(&engine, p0, avarax).unwrap();
    assert!(keywords(&engine, beast).contains(KeywordSet::HASTE));
    assert_eq!(hand_size(&engine, p0), hand_before);
    assert_eq!(pt(&engine, beast), (3, 3));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, avarax, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, beast), (4, 3));
}
