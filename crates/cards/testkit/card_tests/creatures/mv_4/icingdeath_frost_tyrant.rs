//! Icingdeath, Frost Tyrant and its Equipment token, played at a three-player table.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

#[test]
fn icingdeath_frost_tongue_taps_only_a_creature_controlled_by_the_defending_player() {
    let icingdeath = card_index("1dc2c213-72f6-4112-96a8-bf6ad553c707");
    let (me, defender, bystander) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, mountain(), 3)
        .hand(0, &[lightning_bolt(), index::CLOUDSHIFT])
        .battlefield(
            0,
            &[
                icingdeath,
                index::GRIZZLY_BEARS,
                mountain(),
                mountain(),
                mountain(),
                plains(),
            ],
        )
        .battlefield(1, &[index::HILL_GIANT, index::GRIZZLY_BEARS])
        .battlefield(2, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let dragon = on_battlefield(&engine, me, icingdeath).unwrap();
    let attacker = on_battlefield(&engine, me, index::GRIZZLY_BEARS).unwrap();
    let their_creature = on_battlefield(&engine, defender, index::HILL_GIANT).unwrap();
    let alternative = on_battlefield(&engine, defender, index::GRIZZLY_BEARS).unwrap();
    let other_creature = on_battlefield(&engine, bystander, index::HILL_GIANT).unwrap();

    let blink_mana = on_battlefield(&engine, me, plains()).unwrap();
    tap_mana_except(&mut engine, me, blink_mana);
    cast_with_floating(&mut engine, me, lightning_bolt());
    aim_at(&mut engine, me, dragon);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, me, icingdeath).is_some());
    let tokens = tokens_of(&engine, me);
    let [equipment] = tokens.as_slice() else {
        panic!("Icingdeath should leave one Equipment token");
    };
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: *equipment,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, me, attacker);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, attacker), (4, 2));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .unwrap();

    let offered = aim_at(&mut engine, me, their_creature);
    assert_eq!(offered.len(), 2);
    assert!(offered.contains(&alternative));

    let trigger = engine.state().zones.list(ZoneLocation::Stack)[0];
    let attacking_version = engine.state().object(attacker).unwrap().version;
    cast_from_hand(&mut engine, me, index::CLOUDSHIFT);
    aim_at(&mut engine, me, attacker);
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack) == &[trigger]
    });
    let returned = on_battlefield(&engine, me, index::GRIZZLY_BEARS).unwrap();
    assert_ne!(
        engine.state().object(returned).unwrap().version,
        attacking_version
    );
    assert_eq!(pt(&engine, returned), (2, 2));

    let ability = engine.state().object(trigger).unwrap();
    let spec = ability.target_req.unwrap().spec;
    let (objects, players) =
        baylee_engine::eval::stack_retarget_options(engine.state(), ability, &spec);
    assert_eq!(objects.len(), 2);
    assert!(objects.contains(&their_creature));
    assert!(objects.contains(&alternative));
    assert!(players.is_empty());

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, their_creature));
    assert!(!is_tapped(&engine, alternative));
    assert!(!is_tapped(&engine, other_creature));
}
