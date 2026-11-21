//! `cards/artifacts/mv_3/kyren_archive.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kyren_archive() -> CardIndex {
    card_index("f8c7da00-69e1-459d-9b66-6de649c5c511")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn is_face_down(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(baylee_engine::object::Status::FACE_DOWN)
}

fn answer_upkeep_exile(engine: &mut Engine<RegistryLookup>, accept: bool) {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    engine.apply(p0, PlayerAction::YesNo(accept)).unwrap();
    pass_until(engine, stack_is_empty);
}

/// Kyren Archive (f8c7da00-69e1-459d-9b66-6de649c5c511) — {3} Artifact — Book:
/// "At the beginning of your upkeep, you may exile the top card of your library
///  face down."
/// "{5}, Discard your hand, Sacrifice this artifact: Put all cards exiled with
///  this artifact into their owner's hand."
///
/// Exercises accepting twice (cards enter Exile face down), skipping opponent's
/// upkeep, declining once, and then paying {5}, discarding hand, and
/// sacrificing to put all linked exiled cards into hand face up.
#[test]
fn kyren_archive_exiles_face_down_on_upkeep_and_returns_all_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                kyren_archive(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let archive = on_battlefield(&engine, p0, kyren_archive()).unwrap();

    // Turn 1 p0 upkeep: accept -> top card exiled face down.
    answer_upkeep_exile(&mut engine, true);
    let exiled1 = engine.state().zones.list(ZoneLocation::Exile(p0)).clone();
    assert_eq!(exiled1.len(), 1, "first card exiled");
    assert!(
        is_face_down(&engine, exiled1[0]),
        "exiled card is face down"
    );

    // Opponent's upkeep must NOT trigger Kyren Archive.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }) && stack_is_empty(&engine),
        "no trigger during opponent's upkeep"
    );

    // Turn 2 p0 upkeep: decline -> still only 1 card exiled.
    answer_upkeep_exile(&mut engine, false);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        1,
        "declining keeps top of library intact"
    );

    // Turn 3 p0 upkeep: accept -> second card exiled face down.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    answer_upkeep_exile(&mut engine, true);

    let exiled = engine.state().zones.list(ZoneLocation::Exile(p0)).clone();
    assert_eq!(exiled.len(), 2, "two cards now exiled with Kyren Archive");
    assert!(exiled.iter().all(|&id| is_face_down(&engine, id)));

    // Main phase: pay {5}, discard hand, sacrifice Kyren Archive -> both archived cards go to hand.
    reach_main_phase(&mut engine, p0);
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, kyren_archive(), 1);

    assert_eq!(
        engine.state().object(archive).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert!(engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty());

    pass_until(&mut engine, stack_is_empty);

    let hand = engine.state().zones.list(ZoneLocation::Hand(p0));
    assert_eq!(
        hand.len(),
        2,
        "exactly the two linked exiled cards entered hand"
    );
    for &id in &exiled {
        assert!(hand.contains(&id), "archived card {id:?} returned to hand");
        assert!(
            !is_face_down(&engine, id),
            "returned card is face up in hand"
        );
    }
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .is_empty()
    );
}
