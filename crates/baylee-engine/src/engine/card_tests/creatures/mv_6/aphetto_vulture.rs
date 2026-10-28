//! `cards/creatures/mv_6/aphetto_vulture.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SCATHE_ZOMBIES;

#[test]
fn aphetto_vulture_dies_trigger_puts_target_zombie_from_graveyard_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let vulture = card_index("aeba5c63-aa80-4f31-90fb-f404c9a3057d");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[vulture, SCATHE_ZOMBIES, mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vulture_id = on_battlefield(&engine, p0, vulture).unwrap();
    assert!(keywords(&engine, vulture_id).contains(KeywordSet::FLYING));
    let zombie_id = on_battlefield(&engine, p0, SCATHE_ZOMBIES).unwrap();
    bury(&mut engine, &[zombie_id]);
    let zombie_in_gy = in_graveyard(&engine, p0, SCATHE_ZOMBIES).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, vulture_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![zombie_in_gy],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, SCATHE_ZOMBIES).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        SCATHE_ZOMBIES
    );
}
