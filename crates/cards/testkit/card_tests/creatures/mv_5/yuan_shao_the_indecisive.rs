//! `cards/creatures/mv_5/yuan_shao_the_indecisive.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn yuan_shao_the_indecisive() -> CardIndex {
    card_index("2fc5fffa-2eb1-4a3c-af47-3ff8c7df608d")
}

fn shu_general() -> CardIndex {
    card_index("0fef6729-b044-4daf-834a-ac8c9711ad8a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Yuan Shao, the Indecisive (2fc5fffa-2eb1-4a3c-af47-3ff8c7df608d) — `{4}{R}` 2/3
/// Legendary Creature — Human Soldier:
/// - Horsemanship
/// - Each creature you control can't be blocked by more than one creature.
#[test]
fn yuan_shao_caps_blockers_to_one_on_every_creature_you_control_only() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[shu_general(), shu_general()])
        .hand(0, &[yuan_shao_the_indecisive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, yuan_shao_the_indecisive());
    pass_until(&mut engine, stack_is_empty);

    let ys = on_battlefield(&engine, P0, yuan_shao_the_indecisive()).expect("Yuan Shao entered");
    let ally = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, ys), (2, 3));
    assert!(keywords(&engine, ys).contains(KeywordSet::HORSEMANSHIP));

    let p1_generals: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().controller == P1)
        .collect();

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ys, Defender::Player(P1)), (ally, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(p1_generals[0], ally), (p1_generals[1], ally)],
                },
            )
            .is_err(),
        "other creatures you control also cannot be double-blocked"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(p1_generals[0], ys), (p1_generals[1], ys)],
                },
            )
            .is_err(),
        "Yuan Shao himself cannot be double-blocked"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_generals[0], ys), (p1_generals[1], ally)],
            },
        )
        .expect("one blocker per creature is legal");
}

#[test]
fn yuan_shao_does_not_protect_opponent_attackers_from_double_blocking() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[yuan_shao_the_indecisive(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let ys = on_battlefield(&engine, P0, yuan_shao_the_indecisive()).unwrap();
    let p0_bear = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let p1_bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p1_bear, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ys, p1_bear), (p0_bear, p1_bear)],
            },
        )
        .expect("opponent's attacker can still be blocked by two creatures");
}
