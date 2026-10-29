//! `cards/creatures/mv_3/frightshroud_courier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SCATHE_ZOMBIES;

#[test]
fn frightshroud_courier_pumps_target_zombie_while_courier_remains_tapped() {
    let p0 = PlayerId::new(0);
    let courier = card_index("10e3e284-943a-45aa-accb-bb7d2a420098");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[courier, SCATHE_ZOMBIES, swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier_id = on_battlefield(&engine, p0, courier).unwrap();
    let zombie = on_battlefield(&engine, p0, SCATHE_ZOMBIES).unwrap();
    assert_eq!(pt(&engine, zombie), (2, 2));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, courier, 1);
    aim_at(&mut engine, p0, zombie);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, zombie), (4, 4));
    assert!(keywords(&engine, zombie).contains(KeywordSet::FEAR));

    // When Frightshroud Courier untaps on p0's next turn, the bonus ends.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, zombie), (2, 2));
}

#[test]
fn frightshroud_courier_may_stay_tapped_to_keep_the_bonus() {
    let p0 = PlayerId::new(0);
    let courier = card_index("10e3e284-943a-45aa-accb-bb7d2a420098");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[courier, SCATHE_ZOMBIES, swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier_id = on_battlefield(&engine, p0, courier).unwrap();
    let zombie = on_battlefield(&engine, p0, SCATHE_ZOMBIES).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, courier, 1);
    aim_at(&mut engine, p0, zombie);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, zombie), (4, 4));

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
    assert_eq!(pt(&engine, zombie), (4, 4));
}
