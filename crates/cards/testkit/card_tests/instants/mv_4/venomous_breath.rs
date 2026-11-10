//! Venomous Breath — `{3}{G}` Instant (Ice Age)
//!
//! Choose target creature. At this turn's next end of combat, destroy all
//! creatures that blocked or were blocked by it this turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn venomous_breath() -> CardIndex {
    card_index("fc73bd1b-c713-4825-9f2d-f6ae95dddf09")
}

fn wall_of_bone() -> CardIndex {
    card_index("8ffe4986-9e09-4421-ad26-296a4c0df9e4")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn venomous_breath_on_attacker_destroys_creatures_that_blocked_it_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[venomous_breath()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                hill_giant(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[wall_of_bone(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let bystander_p0 = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, p1, wall_of_bone()).unwrap();
    let bystander_p1 = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let spell = in_hand(&engine, p0, venomous_breath()).unwrap();

    // Cast Venomous Breath on Hill Giant in main phase, then attack with Hill Giant.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    // Wall of Bone (1/4) blocks Hill Giant (3/3); both survive combat damage.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Graveyard,
        "Wall of Bone that blocked Hill Giant was destroyed at end of combat"
    );
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Battlefield,
        "Hill Giant itself survives"
    );
    assert_eq!(
        engine.state().object(bystander_p0).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(bystander_p1).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn venomous_breath_on_blocker_destroys_attacker_it_blocked_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[venomous_breath()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), wall_of_bone()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);

    let blocker = on_battlefield(&engine, p0, wall_of_bone()).unwrap();
    let attacker = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let spell = in_hand(&engine, p0, venomous_breath()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    // Cast Venomous Breath on our blocker (Wall of Bone) during declare-blockers step.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![blocker],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Graveyard,
        "Hill Giant that was blocked by Wall of Bone is destroyed at end of combat"
    );
    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Battlefield,
        "Wall of Bone survives"
    );
}
