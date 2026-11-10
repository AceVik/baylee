//! `cards/creatures/mv_4/murk_dwellers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn murk_dwellers() -> CardIndex {
    card_index("00358a89-2058-448b-a2fd-389f639ba2da")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Murk Dwellers (00358a89-2058-448b-a2fd-389f639ba2da) — "Whenever this
/// creature attacks and isn't blocked, it gets +2/+0 until end of combat."
#[test]
fn murk_dwellers_unblocked_gets_plus_two_power_until_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[murk_dwellers()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let dwellers = on_battlefield(&engine, p0, murk_dwellers()).expect("the Dwellers are out");
    assert_eq!(pt(&engine, dwellers), (2, 2));

    let blocks = attack_and_collect_blocks(&mut engine, dwellers, p1);
    assert!(blocks.is_empty(), "nobody to block");
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    assert!(
        !stack_is_empty(&engine),
        "the unblocked trigger is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dwellers), (4, 2), "+2/+0");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 16, "4 damage went through");
    assert_eq!(pt(&engine, dwellers), (2, 2), "the bonus ended with combat");
}

/// Blocked, nothing triggers: it deals its printed 2.
#[test]
fn murk_dwellers_blocked_stays_a_two_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[murk_dwellers()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let dwellers = on_battlefield(&engine, p0, murk_dwellers()).expect("the Dwellers are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    let blocks = attack_and_collect_blocks(&mut engine, dwellers, p1);
    assert!(blocks.iter().any(|b| b.blocker == giant), "{blocks:?}");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, dwellers)],
            },
        )
        .expect("the Giant blocks");
    assert!(
        stack_is_empty(&engine),
        "a blocked attacker triggers nothing"
    );
    assert_eq!(pt(&engine, dwellers), (2, 2));
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant lives")
            .damage,
        2,
        "2 damage, not 4"
    );
    assert!(
        in_graveyard(&engine, p0, murk_dwellers()).is_some(),
        "the 2/2 dies to the 3/3"
    );
    assert_eq!(engine.state().players[1].life, 20);
}
