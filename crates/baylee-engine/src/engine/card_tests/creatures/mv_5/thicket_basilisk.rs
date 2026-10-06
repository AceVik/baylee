//! `cards/creatures/mv_5/thicket_basilisk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thicket Basilisk — "Whenever this creature blocks or becomes blocked by
/// a non-Wall creature, destroy that creature at end of combat." Here, the
/// "becomes blocked" direction, and its one printed exception: a Wall. The
/// blocker is a 2/3 Hurloon Minotaur, not the 2/2 Pearled Unicorn: it
/// survives the Basilisk's 2 combat damage on its own, so only the delayed
/// trigger — not the fight — can be what kills it.
#[test]
fn thicket_basilisk_destroys_a_non_wall_creature_that_blocks_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thicket_basilisk()])
        .battlefield(1, &[hurloon_minotaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let basilisk = on_battlefield(&engine, p0, thicket_basilisk()).expect("seated");
    let blocker = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, basilisk, p1);
    let pairing = blocks
        .iter()
        .find(|b| b.blocker == blocker)
        .unwrap_or_else(|| panic!("the Minotaur may block the Basilisk: {blocks:?}"));
    assert!(pairing.attackers.contains(&basilisk));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, basilisk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(
        engine.state().object(blocker).map(|o| o.zone),
        Some(Zone::Battlefield),
        "2 damage to a 2/3: the fight itself did not kill it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        in_graveyard(&engine, p1, hurloon_minotaur()).is_some(),
        "destroyed at end of combat for blocking the Basilisk, not by the fight"
    );
    assert!(on_battlefield(&engine, p0, thicket_basilisk()).is_some());
}

/// Thicket Basilisk's one exception: a Wall that blocks it is not
/// destroyed.
#[test]
fn thicket_basilisk_does_not_destroy_a_wall_that_blocks_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thicket_basilisk()])
        .battlefield(1, &[wall_of_brambles()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let basilisk = on_battlefield(&engine, p0, thicket_basilisk()).expect("seated");
    let wall = on_battlefield(&engine, p1, wall_of_brambles()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, basilisk, p1);
    let pairing = blocks
        .iter()
        .find(|b| b.blocker == wall)
        .unwrap_or_else(|| panic!("the Wall may still block: {blocks:?}"));
    assert!(pairing.attackers.contains(&basilisk));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, basilisk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        on_battlefield(&engine, p1, wall_of_brambles()).is_some(),
        "a Wall is the one exception the trigger names"
    );
}
