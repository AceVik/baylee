//! `cards/creatures/artifacts/mv_4/needlebug.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn needlebug() -> CardIndex {
    card_index("2d5fba80-c84f-4f84-84e4-9c797ac244fc")
}

/// Needlebug — {4} 2/2 artifact creature with flash. In the opponent's
/// upkeep, where a creature spell without flash is refused, it can be cast
/// and resolves.
#[test]
fn needlebug_is_cast_in_the_opponents_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[needlebug(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("the active player passes");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "now the other seat holds priority: {:?}",
        engine.pending()
    );

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, quiet_creature()).expect("in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: elves })
            .is_err(),
        "the control: a creature without flash is not castable now"
    );
    cast_with_floating(&mut engine, p0, needlebug());
    pass_until(&mut engine, stack_is_empty);
    let bug = on_battlefield(&engine, p0, needlebug()).expect("it resolved");
    assert_eq!(pt(&engine, bug), (2, 2));
    assert_eq!(engine.state().turn.active, p1, "still their turn");
}

/// Protection from artifacts: an artifact creature (Su-Chi, 4/4) attacking
/// into a blocking Needlebug is blocked, its damage to the Needlebug is
/// prevented, and the Needlebug deals its 2 to it.
#[test]
fn needlebug_takes_no_damage_from_an_artifact_creature() {
    let (engine, bug, chi) = they_attack_and_we_block(su_chi(), needlebug());
    assert!(
        on_battlefield(&engine, PlayerId::new(0), needlebug()).is_some(),
        "four damage from an artifact: prevented"
    );
    assert_eq!(engine.state().object(bug).expect("bug").damage, 0);
    assert_eq!(engine.state().object(chi).expect("chi").damage, 2);
}

/// Protection also means an artifact creature cannot block it, while a
/// non-artifact one can.
#[test]
fn needlebug_cannot_be_blocked_by_an_artifact_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[needlebug()])
        .battlefield(1, &[su_chi()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bug = on_battlefield(&engine, p0, needlebug()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, bug, p1);
    assert!(
        blocks.iter().all(|b| !b.attackers.contains(&bug)),
        "Su-Chi is an artifact: {blocks:?}"
    );
    assert!(bears_may_block(needlebug()), "the control: Bears may block");
}
