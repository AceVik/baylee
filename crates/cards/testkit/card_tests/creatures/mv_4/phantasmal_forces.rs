//! `cards/creatures/mv_4/phantasmal_forces.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phantasmal Forces: paying the upkeep keeps a flying 4/1; declining the
/// next payment sacrifices it (CR 118.12a). Flying excludes a ground blocker
/// but permits reach (CR 702.9b).
#[test]
fn alpha_eval_phantasmal_forces_flies_then_demands_each_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let forces = card_index("06a158c6-7e36-49f8-a8e0-a7b7df5fd7ed");
    let spider = card_index("e740ce2f-2134-473c-afa1-1b6d2d1e38ef");
    let mut engine = Duel::new(1005, forest())
        .battlefield(0, &[forces, island()])
        .battlefield(1, &[llanowar_elves(), spider])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
        if *player == p0 && *cost == baylee_core::mana!("{U}"))
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);
    let body = on_battlefield(&engine, p0, forces).unwrap();
    assert_eq!(pt(&engine, body), (4, 1));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    let ground = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let reach = on_battlefield(&engine, p1, spider).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, body, p1);
    assert!(!blocks.iter().any(|b| b.blocker == ground));
    assert!(blocks.iter().any(|b| b.blocker == reach));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    reach_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 16, "four flying damage");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, forces).is_some());
}
