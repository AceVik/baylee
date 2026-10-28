//! `cards/creatures/mv_3/flamestick_courier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn flamestick_courier_pumps_target_goblin_while_courier_remains_tapped() {
    let p0 = PlayerId::new(0);
    let courier = card_index("ea56ca6e-721d-43c7-9a30-4be7c85c67d3");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                courier,
                goblin_berserker(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier_id = on_battlefield(&engine, p0, courier).unwrap();
    let goblin = on_battlefield(&engine, p0, goblin_berserker()).unwrap();
    assert_eq!(pt(&engine, goblin), (2, 2));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, courier, 1);
    aim_at(&mut engine, p0, goblin);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, goblin), (4, 4));
    assert!(keywords(&engine, goblin).contains(KeywordSet::HASTE));

    // When Flamestick Courier untaps on p0's next turn, the bonus ends.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, goblin), (2, 2));
}

#[test]
fn flamestick_courier_may_stay_tapped_to_keep_the_bonus() {
    let p0 = PlayerId::new(0);
    let courier = card_index("ea56ca6e-721d-43c7-9a30-4be7c85c67d3");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                courier,
                goblin_berserker(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier_id = on_battlefield(&engine, p0, courier).unwrap();
    let goblin = on_battlefield(&engine, p0, goblin_berserker()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, courier, 1);
    aim_at(&mut engine, p0, goblin);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, goblin), (4, 4));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![courier_id],
            },
        )
        .unwrap();

    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, goblin), (4, 4));
}
