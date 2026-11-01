//! `cards/sorceries/mv_5/doomskar.rs`, played from hand and through foretell.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::object::Rider;
use baylee_core::mana::ManaCost;

fn doomskar() -> CardIndex {
    card_index("38ece0fa-2c94-4eae-b6ab-74b783862f3e")
}

#[test]
fn doomskar_cast_normally_destroys_creatures_but_not_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[oboro_envoy()])
        .hand(0, &[doomskar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, doomskar());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, oboro_envoy()).is_none());
    assert!(in_graveyard(&engine, p1, oboro_envoy()).is_some());
    assert_eq!(all_on_battlefield(&engine, p0, plains()).len(), 5);
}

/// Foretell is a special action, not suspend: it takes {2}, puts no time
/// counters on the card and hides its characteristics from the opponent.
#[test]
fn doomskar_foretold_is_private_and_cast_for_its_alternative_cost_on_a_later_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 5])
        .battlefield(1, &[oboro_envoy()])
        .hand(0, &[doomskar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = in_hand(&engine, p0, doomskar()).expect("Doomskar is in hand");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.foretellable.contains(&card));
    engine
        .apply(p0, PlayerAction::Foretell { card })
        .expect("foretell pays {2}");

    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    let exiled = engine.state().object(card).expect("the exiled Doomskar");
    assert_eq!(exiled.zone, Zone::Exile);
    assert!(exiled.status.contains(Status::FACE_DOWN));
    assert!(exiled.characteristics().types.is_empty());
    assert_eq!(exiled.counters.get(CounterKind::Time), 0);
    assert!(exiled.may_look_at_exile(p0));
    assert!(!exiled.may_look_at_exile(p1));
    assert!(exiled.riders.iter().any(|rider| matches!(
        rider,
        Rider::Foretold { turn } if *turn == engine.state().turn.number
    )));
    assert!(
        engine.apply(p0, PlayerAction::CastSpell { card }).is_err(),
        "a card foretold this turn cannot yet be cast"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain)
            && at_rest(e, p0)
    });
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&card),
        "turn {}, pool {:?}, riders {:?}, can_cast {:?}, legal {:?}",
        engine.state().turn.number,
        engine.state().players[0].mana_pool,
        engine.state().object(card).unwrap().riders,
        crate::casting::can_cast(engine.state(), &RegistryLookup, p0, card),
        legal
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the later turn offers foretell");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        assert_eq!(
            options.len(),
            1,
            "foretell grants only its alternative cost"
        );
        assert_eq!(options[0].kind, CastModeKind::Foretell);
        assert_eq!(options[0].cost, ManaCost::parse("{1}{W}{W}"));
        engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    }
    let spell = engine.state().object(card).expect("Doomskar on the stack");
    assert_eq!(spell.zone, Zone::Stack);
    assert!(!spell.status.contains(Status::FACE_DOWN));
    assert_eq!(spell.characteristics().types, TypeSet::SORCERY);
    assert!(
        spell
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Foretold { .. }))
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five white mana less {{1}}{{W}}{{W}} leaves two"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, oboro_envoy()).is_none());
    assert_eq!(all_on_battlefield(&engine, p0, plains()).len(), 5);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
    assert!(
        !engine
            .state()
            .object(card)
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Foretold { .. })),
        "the next object is no longer the foretold spell"
    );
}
