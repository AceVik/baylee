//! `cards/creatures/mv_2/wall_of_corpses.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wall_of_corpses() -> CardIndex {
    card_index("16a7895a-ca23-4112-a91f-4b1528e75cea")
}

/// Wall of Corpses (`{1}{B}`, `Coverage::Implemented`): 0/2 Wall with defender
/// and `{B}, Sacrifice this creature: Destroy target creature this creature is
/// blocking.`
#[test]
fn wall_of_corpses_sacrifices_to_destroy_blocked_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears(), llanowar_elves()])
        .battlefield(1, &[wall_of_corpses(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let wall = on_battlefield(&engine, p1, wall_of_corpses()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
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
                blockers: vec![(wall, bear)],
            },
        )
        .unwrap();

    // p0 passes priority in DeclareBlockers step; p1 activates Wall of Corpses.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(options, vec![bear], "only blocked creature is legal target");
    assert!(!options.contains(&elf));
    aim_at(&mut engine, p1, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, wall_of_corpses()).is_some());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
