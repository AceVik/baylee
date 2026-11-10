//! `cards/instants/mv_4/settle_the_wreckage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Reach the target player's optional search after declaring `count` attackers.
fn at_search(count: usize) -> (Engine<RegistryLookup>, Vec<ObjectId>, ObjectId) {
    let settle = card_index("2a2ea189-b663-4f4a-bb23-ff7a4af25f71");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), index::GRIZZLY_BEARS],
        )
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::ELITE_VANGUARD,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[settle])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let own_bear = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let candidates = [
        index::HILL_GIANT,
        index::ELITE_VANGUARD,
        index::GRIZZLY_BEARS,
    ]
    .map(|card| on_battlefield(&engine, p1, card).unwrap());
    let attackers = candidates.into_iter().take(count).collect::<Vec<_>>();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&id| (id, Defender::Player(p0)))
                    .collect(),
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, settle);
    let Pending::ChoosePlayer { player, .. } = engine.pending().clone() else {
        panic!("the spell asks for a target player: {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    (engine, attackers, own_bear)
}

#[test]
fn settle_exiles_only_the_targets_attackers_and_ramps_that_many_tapped_basics() {
    let p1 = PlayerId::new(1);
    let (mut engine, attackers, own_bear) = at_search(2);
    let bystander = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let journal_before = engine.state().journal.entries().len();
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the target chooses basic lands: {:?}", engine.pending());
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (0, 2));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: options.into_iter().take(2).collect(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for attacker in attackers {
        assert_eq!(
            engine.state().object(attacker).map(|o| o.zone),
            Some(Zone::Exile)
        );
    }
    assert_eq!(
        engine.state().object(bystander).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        engine.state().object(own_bear).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    let lands = lands_of(&engine, p1);
    assert_eq!(lands.len(), 2);
    assert!(lands.iter().all(|&id| {
        engine
            .state()
            .object(id)
            .is_some_and(|object| object.controller == p1 && object.status.contains(Status::TAPPED))
    }));
    assert!(
        engine.state().journal.entries()[journal_before..]
            .iter()
            .any(|entry| {
                matches!(entry.event, GameEvent::Shuffled { player, .. } if player == p1)
            })
    );
}

#[test]
fn settle_declining_searches_nothing_and_does_not_shuffle() {
    let p1 = PlayerId::new(1);
    let (mut engine, attackers, _) = at_search(2);
    let library_before = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    let journal_before = engine.state().journal.entries().len();
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(attackers.iter().all(|&id| {
        engine
            .state()
            .object(id)
            .is_some_and(|o| o.zone == Zone::Exile)
    }));
    assert!(lands_of(&engine, p1).is_empty());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)),
        &library_before
    );
    assert!(
        !engine.state().journal.entries()[journal_before..]
            .iter()
            .any(|entry| {
                matches!(entry.event, GameEvent::Shuffled { player, .. } if player == p1)
            })
    );
}

#[test]
fn settle_with_no_attackers_can_search_for_zero_and_then_shuffles() {
    let p1 = PlayerId::new(1);
    let (mut engine, _, _) = at_search(0);
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_some());
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("zero is still a permitted search: {:?}", engine.pending());
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (0, 0));
    let journal_before = engine.state().journal.entries().len();
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(lands_of(&engine, p1).is_empty());
    assert!(
        engine.state().journal.entries()[journal_before..]
            .iter()
            .any(|entry| {
                matches!(entry.event, GameEvent::Shuffled { player, .. } if player == p1)
            })
    );
}
