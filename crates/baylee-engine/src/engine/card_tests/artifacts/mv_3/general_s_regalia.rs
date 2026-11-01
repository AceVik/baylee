//! Tests for General's Regalia.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "cceba6f3-b1c0-45ee-826c-b871d24c7208"
fn general_s_regalia() -> CardIndex {
    card_index("cceba6f3-b1c0-45ee-826c-b871d24c7208")
}

/// {3}: The next time a source of your choice would deal damage to you this
/// turn, that damage is dealt to target creature you control instead.
#[test]
fn general_s_regalia_redirects_next_damage_from_chosen_source_to_target_creature_you_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                hill_giant(),
                general_s_regalia(),
            ],
        )
        .battlefield(1, &[mountain(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let regalia = on_battlefield(&engine, p0, general_s_regalia()).expect("General's Regalia");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("Hill Giant");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("Gray Ogre");

    // Advance to P1's declare-attackers step; P1 attacks P0 with Gray Ogre.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p0))],
            },
        )
        .unwrap();

    // P0 activates General's Regalia targeting Hill Giant and chooses Gray Ogre as the source.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: regalia,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, giant);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    let source = options
        .into_iter()
        .find(|r| r.object == ogre)
        .expect("Gray Ogre offered as damage source");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // P0 declares no blockers; Gray Ogre's 2 combat damage to P0 is redirected to Hill Giant.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
}
