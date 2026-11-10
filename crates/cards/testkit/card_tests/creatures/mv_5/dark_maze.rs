//! Dark Maze — `{4}{U}` 4/5 Creature — Wall (Homelands)
//!
//! Defender
//! `{0}`: Dark Maze can attack this turn as though it didn't have defender.
//! Exile it at the beginning of the next end step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dark_maze() -> CardIndex {
    card_index("44c65424-9c44-4bd8-9022-7ef5bb5b898e")
}

#[test]
fn dark_maze_activates_to_attack_despite_defender_and_exiles_at_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[dark_maze()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let maze = on_battlefield(&engine, p0, dark_maze()).unwrap();
    assert_eq!(pt(&engine, maze), (4, 5));
    assert!(keywords(&engine, maze).contains(KeywordSet::DEFENDER));

    // Activate {0}: Dark Maze can attack this turn and will be exiled at the beginning of the next end step.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: maze,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Attack p1 for 4 combat damage.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(maze, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().object(maze).unwrap().zone, Zone::Battlefield);

    // Advance past the end step into p1's turn: Dark Maze is exiled by the delayed trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|&id| engine
                .state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == dark_maze())),
        "Dark Maze is exiled at the beginning of the next end step"
    );
}

/// Defender holds until the ability is used: with Grizzly Bears in the same
/// menu, Dark Maze is not offered as an attacker.
#[test]
fn dark_maze_cannot_attack_while_its_ability_is_unused() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[dark_maze(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let maze = on_battlefield(&engine, p0, dark_maze()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(attackers.contains(&bears), "the attack menu is open");
    assert!(
        !attackers.contains(&maze),
        "Dark Maze has defender until its {{0}} ability is used"
    );
}
