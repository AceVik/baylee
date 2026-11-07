//! `cards/creatures/mv_1/basking_rootwalla.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn basking_rootwalla() -> CardIndex {
    card_index("b2ec080f-ad5d-4274-91e3-bc6ac89fc3e4")
}

fn prying_eyes() -> CardIndex {
    card_index("06b60766-a1fb-4ed0-93f6-8423008f75b4")
}

#[test]
fn basking_rootwalla_pumps_itself_once_each_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[basking_rootwalla(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rootwalla = on_battlefield(&engine, p0, basking_rootwalla()).expect("Rootwalla seated");
    assert_eq!(pt(&engine, rootwalla), (1, 1));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, basking_rootwalla(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, rootwalla), (3, 3), "+2/+2 until end of turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(rootwalla, 0)),
        "the ability may be activated only once this turn"
    );

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        pt(&engine, rootwalla),
        (1, 1),
        "the pump ended with the turn"
    );
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(rootwalla, 0)),
        "a new turn allows one new activation"
    );
    activate(&mut engine, p0, basking_rootwalla(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, rootwalla), (3, 3));
}

/// Casts Prying Eyes from the current priority and discards Rootwalla plus
/// one other card, stopping at the madness trigger's own question.
fn discard_rootwalla(engine: &mut Engine<RegistryLookup>, p0: PlayerId) -> ObjectId {
    tap_all_mana(engine, p0);
    cast_with_floating(engine, p0, prying_eyes());
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let rootwalla = in_hand(engine, p0, basking_rootwalla()).expect("Rootwalla is in hand");
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != rootwalla)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rootwalla, other],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: baylee_engine::choice::YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == rootwalla
        )
    });
    assert_eq!(
        engine.state().object(rootwalla).map(|object| object.zone),
        Some(Zone::Exile),
        "madness replaced the discard's destination with exile"
    );
    rootwalla
}

#[test]
fn basking_rootwalla_madness_casts_for_zero_during_an_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[basking_rootwalla(), prying_eyes()])
        .battlefield(0, &[island(); 6])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected opponent priority, got {:?}", engine.pending());
    };
    assert_eq!(player, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let rootwalla = discard_rootwalla(&mut engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, basking_rootwalla()).is_some() && stack_is_empty(e)
    });

    assert_eq!(engine.state().turn.active, p1, "the cast ignored timing");
    assert_eq!(
        on_battlefield(&engine, p0, basking_rootwalla()),
        Some(rootwalla),
        "the discarded incarnation was cast for madness {{0}}"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        mine(&engine, p0, basking_rootwalla(), Zone::Graveyard).len(),
        0,
        "a successfully cast card does not take the graveyard fallback"
    );
}

#[test]
fn basking_rootwalla_declining_madness_puts_the_same_card_into_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[basking_rootwalla(), prying_eyes()])
        .battlefield(0, &[island(); 6])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rootwalla = discard_rootwalla(&mut engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().object(rootwalla).map(|object| object.zone),
        Some(Zone::Graveyard)
    );
    assert!(on_battlefield(&engine, p0, basking_rootwalla()).is_none());
    assert!(mine(&engine, p0, basking_rootwalla(), Zone::Exile).is_empty());
}
