//! `cards/creatures/mv_3/rootwalla.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rootwalla — {2}{G} 2/2 Lizard: "{1}{G}: This creature gets +2/+2 until end
/// of turn. Activate only once each turn."
///
/// Four Forests are tapped so that the {1}{G} is a real payment and — the half
/// that makes the limit mean anything — two green are still floating once the
/// pump has resolved, which is exactly what a second activation would cost.
/// The ability's absence from `LegalActions::abilities` is therefore the printed
/// once-a-turn clause and not `can_afford` reading an empty pool. The following
/// turn is the control: the Lizard is a printed 2/2 again once the pump's
/// until-end-of-turn has expired, and the same line is offered a second time —
/// "once each turn" is a limit and not a lost ability.
#[test]
fn rootwalla_pumps_itself_once_a_turn_and_the_pump_expires_with_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), rootwalla()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lizard = on_battlefield(&engine, p0, rootwalla()).expect("the Lizard is on the table");
    assert_eq!(
        pt(&engine, lizard),
        (2, 2),
        "the printed body, before anything asks"
    );

    // Mana first: the offer is read off the pool, and {1}{G} is two mana.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, and the Lizard itself taps for nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lizard, 0)),
        "four green pays {{1}}{{G}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, rootwalla(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, lizard),
        (4, 4),
        "+2/+2, and the pump is the whole of it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{G}} came out of the pool and two green are left"
    );

    // The limit with the price still covered: two mana is exactly a second
    // activation's cost, so a missing entry here cannot be an unaffordable
    // ability being withheld.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(lizard, 0)),
        "\"Activate only once each turn\": {:?}",
        legal.abilities
    );

    // And a limit is not a lost ability: the next turn hands the Lizard back
    // its printed body and its {1}{G}.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, lizard),
        (2, 2),
        "\"until end of turn\" is exactly one turn long"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the Forests untapped and were tapped again"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lizard, 0)),
        "once *each* turn, so the new turn offers the line again: {:?}",
        legal.abilities
    );
}
