//! `cards/creatures/artifacts/mv_3/night_market_guard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn night_market_guard() -> CardIndex {
    card_index("3a7a27fe-60f0-4a21-a1c1-5fef8a528391")
}

/// Night Market Guard — "This creature can block an additional creature each
/// combat": against two attackers the block offer names it with room for two,
/// and a declaration with it blocking both is accepted.
#[test]
fn night_market_guard_blocks_two_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[night_market_guard()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, night_market_guard()).expect("seated");
    assert_eq!(pt(&engine, guard), (3, 1));
    let attackers = all_on_battlefield(&engine, p1, quiet_creature());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(p0)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { capacity, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        capacity.iter().any(|c| c.blocker == guard && c.most == Some(2)),
        "the guard is offered two blocks: {capacity:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(guard, attackers[0]), (guard, attackers[1])],
                },
            )
            .is_ok(),
        "one Guard blocks both attackers"
    );
}

/// Without the Guard, a creature blocks one attacker: two blocks by the same
/// plain creature are refused.
#[test]
fn an_ordinary_creature_cannot_block_two_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let blocker = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let attackers = all_on_battlefield(&engine, p1, quiet_creature());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(p0)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, attackers[0]), (blocker, attackers[1])],
                },
            )
            .is_err()
    );
}
