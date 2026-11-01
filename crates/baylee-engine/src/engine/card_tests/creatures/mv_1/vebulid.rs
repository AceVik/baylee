#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn vebulid() -> CardIndex {
    card_index("a887def5-c9e1-4cbb-8b08-79b7bd808deb")
}

#[test]
fn vebulid_enters_with_counter_and_dies_at_end_of_combat_when_attacking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[vebulid()])
        .battlefield(0, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, vebulid());
    pass_until(&mut engine, stack_is_empty);

    let veb = on_battlefield(&engine, p0, vebulid()).expect("vebulid on bf");
    assert_eq!(pt(&engine, veb), (1, 1));

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, veb), (2, 2));

    let _ = attack_and_collect_blocks(&mut engine, veb, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[1].life, 18);
    assert!(on_battlefield(&engine, p0, vebulid()).is_none());
    assert!(in_graveyard(&engine, p0, vebulid()).is_some());
}

#[test]
fn vebulid_may_decline_a_counter_and_is_destroyed_after_blocking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let ground_attacker = baylee_core::generated::index::CROOKSHANK_KOBOLDS;
    let mut engine = lk::game_with(SEED, &[vebulid()], &[ground_attacker], &[], &[], |e| {
        lk::yes_no(e, false);
    });
    let veb = lk::obj(&engine, p0, vebulid());
    assert_eq!(pt(&engine, veb), (1, 1), "upkeep counter is optional");
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, veb), (1, 1), "opponent upkeep adds no counter");
    let attacker = lk::obj(&engine, p1, ground_attacker);
    lk::attack(&mut engine, &[attacker]);
    lk::block(&mut engine, &[(veb, attacker)]);
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, p0, vebulid()).is_some());
    lk::through_combat(&mut engine);
    assert!(in_graveyard(&engine, p0, vebulid()).is_some());
}
