//! `cards/instants/mv_1/lava_dart.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lava_dart() -> CardIndex {
    card_index("e48891e3-30a2-4fc8-a858-cec33c6e4ab5")
}

fn put_lava_dart_in_graveyard(engine: &mut Engine<RegistryLookup>, owner: PlayerId) -> ObjectId {
    let dart = in_hand(engine, owner, lava_dart()).unwrap();
    engine
        .dev_state_mut(owner)
        .unwrap()
        .move_object(
            dart,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();
    // The dev-state door intentionally leaves a priority offer's cached legal
    // actions alone; this fixture changed which zone the spell is cast from.
    engine.refresh_offer();
    dart
}

fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, dart: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: dart })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// The ordinary spell costs exactly {R} and deals one damage to either kind of
/// any target. It does not flash itself back after an ordinary cast.
#[test]
fn lava_dart_costs_one_red_and_deals_one_to_a_player_or_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lava_dart(), lava_dart()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2
    );
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, lava_dart());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(in_graveyard(&engine, p0, lava_dart()).is_some());

    cast_from_hand(&mut engine, p0, lava_dart());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(life_of(&engine, p1), 19);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .filter(|&&id| {
                engine
                    .state()
                    .object(id)
                    .is_some_and(|object| object.card.is_some_and(|card| card.index == lava_dart()))
            })
            .count(),
        2,
        "both ordinary casts go to the graveyard, not exile"
    );
}

/// Flashback is an alternative cost, not an additional red mana payment. Only
/// the caster's own Mountain pays it, after the target has been announced.
#[test]
fn lava_dart_flashback_sacrifices_an_own_mountain_then_exiles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), forest()])
        .battlefield(1, &[mountain(), grizzly_bears()])
        .hand(0, &[lava_dart()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dart = put_lava_dart_in_graveyard(&mut engine, p0);
    let own_mountain = on_battlefield(&engine, p0, mountain()).unwrap();
    let forest = on_battlefield(&engine, p0, forest()).unwrap();
    let opposing_mountain = on_battlefield(&engine, p1, mountain()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    start_flashback(&mut engine, p0, dart);
    aim_at(&mut engine, p0, bears);
    assert_eq!(
        engine.state().object(own_mountain).unwrap().zone,
        Zone::Battlefield,
        "the target is chosen before the sacrifice is paid"
    );
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the Mountain sacrifice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![own_mountain]);
    for invalid in [forest, opposing_mountain] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![invalid],
                    },
                )
                .is_err(),
            "a Forest or an opponent's Mountain cannot pay this cost"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![own_mountain],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(own_mountain).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(dart).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(life_of(&engine, p1), 20);
    assert_eq!(engine.state().object(dart).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, lava_dart()).is_none());
}

/// Neither another land type nor an opponent's Mountain enables flashback.
#[test]
fn lava_dart_flashback_is_not_offered_without_an_own_mountain() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[forest()])
        .battlefield(1, &[mountain()])
        .hand(0, &[lava_dart()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dart = put_lava_dart_in_graveyard(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&dart));
    assert!(!legal.payable.contains(&dart));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: dart })
            .is_err()
    );
    assert_eq!(engine.state().object(dart).unwrap().zone, Zone::Graveyard);
}

/// The Mountain remains sacrificed when the sole target leaves before
/// resolution. No other creature or player becomes a replacement target.
#[test]
fn lava_dart_flashback_does_not_retarget_and_still_exiles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lava_dart()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dart = put_lava_dart_in_graveyard(&mut engine, p0);
    let mountain = on_battlefield(&engine, p0, mountain()).unwrap();
    let own_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let target = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    start_flashback(&mut engine, p0, dart);
    aim_at(&mut engine, p0, target);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mountain],
            },
        )
        .unwrap();
    bury(&mut engine, &[target]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(own_bears).unwrap().damage, 0);
    assert_eq!(life_of(&engine, p0), 20);
    assert_eq!(life_of(&engine, p1), 20);
    assert_eq!(
        engine.state().object(mountain).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(dart).unwrap().zone, Zone::Exile);
}
