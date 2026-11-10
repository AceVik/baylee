//! `cards/creatures/mv_5/woodripper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

#[test]
fn woodripper_removes_fade_counter_to_destroy_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("d0653734-9eb7-48c6-9042-65ffdd50fd2a")])
        .battlefield(0, &[forest(); 6])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    cast_from_hand(
        &mut engine,
        p0,
        card_index("d0653734-9eb7-48c6-9042-65ffdd50fd2a"),
    );
    pass_until(&mut engine, stack_is_empty);

    let woodripper = on_battlefield(
        &engine,
        p0,
        card_index("d0653734-9eb7-48c6-9042-65ffdd50fd2a"),
    )
    .unwrap();
    assert_eq!(
        engine
            .state()
            .object(woodripper)
            .unwrap()
            .counters
            .get(counters::FADE),
        3
    );

    tap_all_mana(&mut engine, p0);
    activate(
        &mut engine,
        p0,
        card_index("d0653734-9eb7-48c6-9042-65ffdd50fd2a"),
        1,
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(woodripper)
            .unwrap()
            .counters
            .get(counters::FADE),
        2
    );
    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
}
