//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn relic_bind_chooses_its_mode_and_target_when_the_trigger_is_stacked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for mode in [0, 1] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::ISLAND, index::ISLAND, index::ISLAND])
            .battlefield(1, &[index::MIND_STONE])
            .hand(0, &[index::RELIC_BIND])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let stone = on_battlefield(&e, p1, index::MIND_STONE).unwrap();
        cast_from_hand(&mut e, p0, index::RELIC_BIND);
        aim_at(&mut e, p0, stone);
        pass_until(&mut e, stack_is_empty);
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
        );
        activate(&mut e, p1, index::MIND_STONE, 0);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
        assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[1].life, if mode == 0 { 19 } else { 21 });
    }
}

/// "Enchant artifact an opponent controls": the caster's own artifact and an
/// opponent's non-artifact are not offered when it is cast. And mode 0's
/// "target player or planeswalker" offers the two players and no object at
/// all while no planeswalker is on the table, a Grizzly Bears included.
#[test]
fn relic_bind_enchants_only_an_opponents_artifact_and_its_damage_mode_names_no_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                quiet_artifact(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::MIND_STONE, index::GRIZZLY_BEARS])
        .hand(0, &[index::RELIC_BIND])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let stone = on_battlefield(&e, p1, index::MIND_STONE).unwrap();
    let my_artifact = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    let my_bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let their_bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut e, p0, index::RELIC_BIND);
    let enchantable = aim_at(&mut e, p0, stone);
    assert!(
        enchantable.contains(&stone),
        "an opponent's artifact is enchantable"
    );
    assert!(
        !enchantable.contains(&my_artifact),
        "\"Enchant artifact an opponent controls\": the caster's own artifact is not offered"
    );
    assert!(
        !enchantable.contains(&my_bear) && !enchantable.contains(&their_bear),
        "a creature is no artifact"
    );
    pass_until(&mut e, stack_is_empty);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );

    activate(&mut e, p1, index::MIND_STONE, 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!(
            "expected the damage mode's target choice, got {:?}",
            e.pending()
        )
    };
    assert!(
        options.is_empty(),
        "\"target player or planeswalker\": with no planeswalker on the table no object is offered, not even a creature: {options:?}"
    );
    let mut players = player_options;
    players.sort();
    assert_eq!(players, vec![p0, p1], "either player may be the target");
}
