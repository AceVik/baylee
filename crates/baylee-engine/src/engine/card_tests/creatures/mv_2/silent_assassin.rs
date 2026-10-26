//! `cards/creatures/mv_2/silent_assassin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "05b07f1f-42e1-41fb-8858-5e4a311c4ed9"
fn silent_assassin() -> CardIndex {
    card_index("05b07f1f-42e1-41fb-8858-5e4a311c4ed9")
}

/// `Silent Assassin` (`Coverage::Generated`):
/// "{3}{B}: Destroy target blocking creature at end of combat."
///
/// Verifies that activating `Silent Assassin` targeting a blocking creature
/// leaves the creature on the battlefield during combat damage and destroys it
/// at the beginning of the end of combat step.
#[test]
fn silent_assassin_destroys_target_blocking_creature_at_end_of_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9608, swamp())
        .battlefield(
            0,
            &[
                silent_assassin(),
                index::RAGING_GOBLIN,
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let assassin = on_battlefield(&engine, p0, silent_assassin()).expect("P0 Silent Assassin");
    let goblin = on_battlefield(&engine, p0, index::RAGING_GOBLIN).expect("P0 Raging Goblin");
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).expect("P1 Hill Giant");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(goblin, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, goblin)],
            },
        )
        .unwrap();

    // During declare blockers step, P0 activates Silent Assassin targeting the blocking Hill Giant.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: assassin,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, index::HILL_GIANT).is_some(),
        "Hill Giant is not destroyed until end of combat"
    );

    // Advance past combat into second main phase; at end of combat Hill Giant is destroyed.
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, p1, index::HILL_GIANT).is_none(),
        "Silent Assassin's delayed trigger destroys the blocking creature at end of combat"
    );
}
