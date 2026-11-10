//! `cards/artifacts/mv_4/primal_amulet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Primal Amulet` // `Primal Wellspring` (`Coverage::Partial`):
/// "Instant and sorcery spells you cast cost `{{1}}` less to cast. Whenever you cast an instant
/// or sorcery spell, put a charge counter on this artifact. Then if there are four or more charge
/// counters on it, you may remove those counters and transform it. // `{{T}}`: Add one mana of any color.
/// When that mana is spent to cast an instant or sorcery spell, copy that spell and you may choose
/// new targets for the copy."
///
/// Under `Coverage::Partial`, the cost reduction, four-counter transform, and copy rider are omitted,
/// leaving the `Trigger::SpellCast` trigger that places a charge counter when you cast an instant or sorcery.
/// The test casts `dark_ritual()` from hand, verifies that `Primal Amulet` receives a `CounterKind::Charge` counter,
/// and verifies that an opponent's instant spell does not trigger the controller's `Primal Amulet`.
#[test]
fn primal_amulet_gains_charge_counter_on_your_instant_cast_and_ignores_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(303, swamp())
        .battlefield(0, &[primal_amulet(), swamp()])
        .hand(0, &[dark_ritual()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet =
        on_battlefield(&engine, p0, primal_amulet()).expect("Primal Amulet on battlefield");
    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        0,
        "starts with zero charge counters"
    );

    // p0 casts Dark Ritual; Primal Amulet triggers and puts a charge counter on itself.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        1,
        "gained one charge counter after casting an instant spell"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "Dark Ritual resolved and added three black mana"
    );

    // Advance to p1's turn and have p1 cast an instant.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        1,
        "opponent casting an instant does not trigger your Primal Amulet"
    );
}

/// Casts the one Dark Ritual in p0's hand with the Amulet already holding
/// `charges` charge counters, and runs the Amulet's trigger up to the point where
/// either the "you may remove those counters and transform it" question is
/// asked or the stack is empty.
fn cast_an_instant_over(charges: u16) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(303, swamp())
        .battlefield(0, &[primal_amulet(), swamp()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let amulet =
        on_battlefield(&engine, p0, primal_amulet()).expect("Primal Amulet on battlefield");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(amulet)
        .unwrap()
        .counters
        .add(CounterKind::Charge, charges);
    assert_eq!(counters_on(&engine, amulet, CounterKind::Charge), charges);
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::YesNo { .. })
    });
    (engine, amulet)
}

/// "Then if there are **four or more** charge counters on it": the third counter
/// is not enough. Nothing is asked and the Amulet stays what it is.
#[test]
fn primal_amulet_with_three_charge_counters_asks_nothing() {
    let (engine, amulet) = cast_an_instant_over(2);
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "two counters and one more is three: no transform question, got {:?}",
        engine.pending()
    );
    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        3,
        "the cast put one counter on the Amulet, and nothing took them off"
    );
    assert_eq!(
        engine.state().object(amulet).unwrap().face_index,
        0,
        "three counters leave it Primal Amulet"
    );
}

/// The fourth counter is enough: the question is asked, declining keeps all four
/// counters, and accepting removes them all and turns the Amulet over.
#[test]
fn primal_amulet_with_the_fourth_charge_counter_offers_the_transform() {
    let p0 = PlayerId::new(0);
    for accept in [false, true] {
        let (mut engine, amulet) = cast_an_instant_over(3);
        assert!(
            matches!(engine.pending(), Pending::YesNo { .. }),
            "three counters and one more is four: the transform is offered, got {:?}",
            engine.pending()
        );
        assert_eq!(
            counters_on(&engine, amulet, CounterKind::Charge),
            4,
            "the question comes with the fourth counter on the Amulet"
        );
        engine.apply(p0, PlayerAction::YesNo(accept)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            counters_on(&engine, amulet, CounterKind::Charge),
            if accept { 0 } else { 4 },
            "accepting removes those counters, declining keeps them (accept = {accept})"
        );
        assert_eq!(
            engine.state().object(amulet).unwrap().face_index,
            u8::from(accept),
            "it transforms exactly when the player says so (accept = {accept})"
        );
    }
}
