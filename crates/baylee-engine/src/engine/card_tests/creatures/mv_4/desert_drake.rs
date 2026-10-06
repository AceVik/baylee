//! `cards/creatures/mv_4/desert_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Desert Drake` is a 2/2 Drake costing `{3}{R}` under `Coverage::Implemented` with flying.
/// When cast from hand off four Mountains, it enters the battlefield with its printed 2/2 characteristics
/// and the flying keyword. When it attacks on a subsequent turn, an opponent controlling only a ground creature
/// without reach cannot legally assign it as a blocker.
#[test]
fn desert_drake_enters_with_flying_and_cannot_be_blocked_by_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[desert_drake()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, desert_drake());
    pass_until(&mut engine, stack_is_empty);

    let drake = on_battlefield(&engine, p0, desert_drake()).expect("Desert Drake resolved");
    assert_eq!(pt(&engine, drake), (2, 2), "printed body is 2/2");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "Desert Drake has flying"
    );

    // Advance to next turn so Desert Drake can attack without summoning sickness.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let blocks = attack_and_collect_blocks(&mut engine, drake, p1);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls elves");
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == elves && b.attackers.contains(&drake)),
        "ground creature without reach cannot block flying Desert Drake: {blocks:?}"
    );
}
