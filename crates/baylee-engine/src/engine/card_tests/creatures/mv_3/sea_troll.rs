//! Sea Troll — `{2}{U}` 2/1 Creature — Troll (Homelands)
//!
//! `{U}`: Regenerate this creature. Activate only if this creature blocked or was blocked by a blue
//! creature this turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sea_troll() -> CardIndex {
    card_index("feabe30a-3c28-43a1-b538-b0207758520c")
}

#[test]
fn sea_troll_cannot_regenerate_before_blocking_and_regenerates_after_blocked_by_blue_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[sea_troll(), island(), island()])
        .battlefield(1, &[phantom_monster()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let troll = on_battlefield(&engine, p0, sea_troll()).unwrap();
    let monster = on_battlefield(&engine, p1, phantom_monster()).unwrap();

    // Before combat, Sea Troll has neither blocked nor been blocked by a blue creature this turn.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: troll,
                    ability_index: 0,
                },
            )
            .is_err(),
        "Sea Troll cannot activate its regeneration ability before blocking or being blocked by a blue creature"
    );

    // Attack with Sea Troll; p1 blocks with 3/3 blue Phantom Monster.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(troll, Defender::Player(p1))],
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
                blockers: vec![(monster, troll)],
            },
        )
        .unwrap();

    // Now in DeclareBlockers step after being blocked by a blue creature, Sea Troll can activate {U}: Regenerate!
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(troll).unwrap().zone,
        Zone::Battlefield,
        "Sea Troll regenerated from lethal combat damage"
    );
}
