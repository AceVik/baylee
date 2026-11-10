//! `cards/creatures/mv_2/withered_wretch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn withered_wretch() -> CardIndex {
    card_index("29e13ec1-a239-4217-a2bd-24ff90745ec0")
}

fn activate_at(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    activate(engine, player, withered_wretch(), 0);
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
}

/// Each activation costs exactly one generic mana, does not tap the Wretch,
/// and may choose a card in either player's graveyard.
#[test]
fn withered_wretch_exiles_from_either_graveyard_for_one_generic_mana_each() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[withered_wretch(), swamp(), swamp()])
        .battlefield(1, &[forest()])
        .hand(0, &[llanowar_elves()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = hand_to_graveyard(&mut engine, p0, llanowar_elves());
    let theirs = hand_to_graveyard(&mut engine, p1, grizzly_bears());
    let wretch = on_battlefield(&engine, p0, withered_wretch()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, withered_wretch(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the graveyard target, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![mine, theirs]);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![land],
                    players: vec![],
                },
            )
            .is_err(),
        "a card on the battlefield is not a card in a graveyard"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(theirs).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(mine).unwrap().zone, Zone::Graveyard);
    assert!(!is_tapped(&engine, wretch));

    activate_at(&mut engine, p0, mine);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(mine).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().object(wretch).unwrap().zone,
        Zone::Battlefield
    );
}

/// A mandatory target means an empty pair of graveyards offers no activation.
#[test]
fn withered_wretch_needs_a_card_in_a_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[withered_wretch(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wretch = on_battlefield(&engine, p0, withered_wretch()).unwrap();
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(wretch, 0)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: wretch,
                    ability_index: 0,
                },
            )
            .is_err()
    );
}

/// If the announced card leaves its graveyard, no other card replaces it.
#[test]
fn withered_wretch_does_not_retarget_when_the_card_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[withered_wretch(), swamp(), swamp()])
        .hand(1, &[grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = hand_to_graveyard(&mut engine, p1, grizzly_bears());
    let other = hand_to_graveyard(&mut engine, p1, gray_ogre());
    tap_all_mana(&mut engine, p0);
    activate_at(&mut engine, p0, target);
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            target,
            ZoneLocation::Hand(p1),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();
    engine.refresh_offer();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(other).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(on_battlefield(&engine, p0, withered_wretch()).is_some());
    assert_eq!(
        engine.state().object(target).map(|object| object.zone),
        Some(Zone::Hand),
        "the departed card was not exiled by the ability"
    );
}
