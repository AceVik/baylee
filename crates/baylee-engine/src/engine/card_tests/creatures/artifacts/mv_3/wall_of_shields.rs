//! `cards/creatures/artifacts/mv_3/wall_of_shields.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wall_of_shields() -> CardIndex {
    card_index("530514d9-7274-4835-a4b5-6dccef4da758")
}

/// Wall of Shields — defender, banding, 0/4. It cannot attack.
#[test]
fn wall_of_shields_has_defender_and_banding_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[wall_of_shields(), quiet_creature()]);
    let wall = on_battlefield(&engine, p0, wall_of_shields()).expect("seated");
    let plain = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 4));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    assert!(keywords(&engine, wall).contains(KeywordSet::BANDING));

    let offered = attackers_offered(&mut engine);
    assert!(offered.contains(&plain));
    assert!(!offered.contains(&wall));
}

/// Banding: when a creature with banding you control blocks, "you divide that
/// creature's combat damage, not its controller": seat 0 blocks seat 1's
/// Wicker Witch with the Wall and a plain 1/1, and seat 0 chooses where its
/// 3 damage goes, saving the 1/1.
#[test]
fn wall_of_shields_lets_its_controller_divide_the_attackers_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Wicker Witch, a 3/1.
    let witch = card_index("cc2f945f-9048-4978-b61f-49eefb3d94d7");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wall_of_shields(), quiet_creature()])
        .battlefield(1, &[witch])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_shields()).expect("seated");
    let small = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let attacker = on_battlefield(&engine, p1, witch).expect("seated");

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
                blockers: vec![(wall, attacker), (small, attacker)],
            },
        )
        .unwrap();

    // The defending player is asked how the attacker's damage divides.
    for _ in 0..4 {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
                || (e.state().turn.phase == Phase::SecondMain && stack_is_empty(e))
        });
        let Pending::ChooseNumber {
            player,
            max,
            reason: crate::choice::NumberPrompt::CombatDamage { recipient, .. },
            ..
        } = engine.pending().clone()
        else {
            break;
        };
        assert_eq!(
            player, p0,
            "the Wall's controller divides, not the attacker's"
        );
        let amount = if recipient == small { 0 } else { max };
        engine
            .apply(p0, PlayerAction::ChooseNumber(amount))
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the 1/1 was spared"
    );
    assert!(
        on_battlefield(&engine, p0, wall_of_shields()).is_some(),
        "the 0/4 took 3"
    );
    assert!(
        on_battlefield(&engine, p1, witch).is_none(),
        "the 1/1 killed the 3/1"
    );
}
