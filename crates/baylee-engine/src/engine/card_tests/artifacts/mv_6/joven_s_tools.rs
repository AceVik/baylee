//! Joven's Tools — `{6}` Artifact (Homelands)
//!
//! `{4}`, `{T}`: Target creature can't be blocked this turn except by Walls.

#[allow(clippy::wildcard_imports)]
use super::*;

fn joven_s_tools() -> CardIndex {
    card_index("b1935140-b038-4a81-98f7-28d2c137124a")
}

#[test]
fn joven_s_tools_makes_target_creature_unblockable_except_by_walls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                joven_s_tools(),
                card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
                gray_ogre(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(
            1,
            &[
                card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
                card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7"),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tools = on_battlefield(&engine, p0, joven_s_tools()).unwrap();
    let attacker = on_battlefield(
        &engine,
        p0,
        card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
    )
    .unwrap();
    let untargeted = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let non_wall = on_battlefield(
        &engine,
        p1,
        card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
    )
    .unwrap();
    let wall = on_battlefield(
        &engine,
        p1,
        card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7"),
    )
    .unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tools,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![attacker],
                players: vec![],
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
                attackers: vec![
                    (attacker, Defender::Player(p1)),
                    (untargeted, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(non_wall, attacker)],
                },
            )
            .is_err(),
        "non-Wall creature cannot block target of Joven's Tools"
    );
    // Only the target is restricted: the other attacker may be blocked by
    // the non-Wall creature, and the Wall may still block the target.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(non_wall, untargeted), (wall, attacker)],
            },
        )
        .expect("the untargeted attacker takes a non-Wall blocker, the target a Wall");
}
