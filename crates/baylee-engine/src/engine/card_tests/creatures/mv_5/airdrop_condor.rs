//! `cards/creatures/mv_5/airdrop_condor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn airdrop_condor_sacrifices_a_goblin_to_deal_its_power_in_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let condor = card_index("25fc274f-d5af-4174-b8e8-c2e65bb4c13f");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[condor, goblin_berserker(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let condor_id = on_battlefield(&engine, p0, condor).unwrap();
    assert!(keywords(&engine, condor_id).contains(KeywordSet::FLYING));
    let goblin = on_battlefield(&engine, p0, goblin_berserker()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, condor, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, goblin_berserker()).is_none());
    assert_eq!(engine.state().players[1].life, 18);
}
