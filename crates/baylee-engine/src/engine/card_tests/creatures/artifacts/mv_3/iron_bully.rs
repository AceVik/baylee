//! `cards/creatures/artifacts/mv_3/iron_bully.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn iron_bully() -> CardIndex {
    card_index("7d78155c-c63f-48a3-86e5-12b80400d8de")
}

/// Iron Bully — "When this creature enters, put a +1/+1 counter on target
/// creature": any creature is a legal target, and the one chosen grows.
#[test]
fn iron_bully_puts_a_counter_on_the_creature_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[iron_bully()])
        .battlefield(0, &[forest(), forest(), forest(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("seated");

    cast_from_hand(&mut engine, p0, iron_bully());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, mine);
    let bully = on_battlefield(&engine, p0, iron_bully()).expect("it is out");
    assert!(options.contains(&mine) && options.contains(&theirs));
    assert!(options.contains(&bully), "\"target creature\" includes itself");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, mine, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, mine), (2, 2));
    assert_eq!(counters_on(&engine, theirs, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, bully), (1, 1));
}

/// Menace: one blocker alone is refused, two together are accepted.
#[test]
fn iron_bully_needs_two_blockers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[iron_bully()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bully = on_battlefield(&engine, p0, iron_bully()).expect("seated");
    assert!(keywords(&engine, bully).contains(KeywordSet::MENACE));
    let blockers = all_on_battlefield(&engine, p1, quiet_creature());
    assert_eq!(blockers.len(), 2);

    attack_to_blocks(&mut engine, &[bully]);
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blockers[0], bully)],
                },
            )
            .is_err(),
        "a single blocker is not enough"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blockers[0], bully), (blockers[1], bully)],
                },
            )
            .is_ok(),
        "two together may block it"
    );
}
