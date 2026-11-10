//! `cards/artifacts/mv_4/acidic_dagger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, WALL_OF_WOOD};

fn acidic_dagger() -> CardIndex {
    card_index("bf034a20-e362-4f38-9bf9-857777c62598")
}

#[test]
fn acidic_dagger_destroys_damaged_non_wall_and_sacrifices_when_target_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                acidic_dagger(),
                GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dagger = on_battlefield(&engine, p0, acidic_dagger()).unwrap();
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, acidic_dagger(), 0);
    aim_at(&mut engine, p0, attacker);
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
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    // Combat damage: Grizzly Bears (2/2) dies to Hill Giant (3/3), and Acidic
    // Dagger's granted trigger destroys Hill Giant and its delayed LTB trigger
    // sacrifices Acidic Dagger.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Graveyard,
        "Hill Giant should be destroyed after taking combat damage from the targeted creature"
    );
    assert_eq!(
        engine.state().object(dagger).unwrap().zone,
        Zone::Graveyard,
        "Acidic Dagger should be sacrificed when the targeted creature leaves the battlefield this turn"
    );
}

/// p0 attacks with `attacker`; p1 blocks it with `blocker`. Stops with p0
/// holding priority in the declare blockers step.
fn attack_and_get_blocked(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
    blocker: ObjectId,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
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
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
}

#[test]
fn acidic_dagger_spares_a_wall_and_stays_while_its_target_survives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                acidic_dagger(),
                GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dagger = on_battlefield(&engine, p0, acidic_dagger()).unwrap();
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let wall = on_battlefield(&engine, p1, WALL_OF_WOOD).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, acidic_dagger(), 0);
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);
    attack_and_get_blocked(&mut engine, attacker, wall);
    // Grizzly Bears (2) hurts Wall of Wood (0/3), which is a Wall: not destroyed.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    let wall_object = engine.state().object(wall).unwrap();
    assert_eq!(
        wall_object.zone,
        Zone::Battlefield,
        "a Wall is never destroyed"
    );
    assert_eq!(wall_object.damage, 2, "the damage was dealt");
    // The targeted creature never left the battlefield: the Dagger stays.
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(dagger).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn acidic_dagger_cannot_be_activated_once_blockers_are_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                acidic_dagger(),
                GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dagger = on_battlefield(&engine, p0, acidic_dagger()).unwrap();
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    attack_and_get_blocked(&mut engine, attacker, blocker);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(dagger, 0)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: dagger,
                    ability_index: 0,
                },
            )
            .is_err(),
        "activate only before blockers are declared"
    );
}
