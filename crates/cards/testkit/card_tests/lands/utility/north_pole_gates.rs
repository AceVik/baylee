//! `cards/lands/utility/north_pole_gates.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `North Pole Gates` is a utility land under `Coverage::Implemented` that enters tapped,
/// taps for `{W}` or `{U}`, and has "{4}, {T}, Sacrifice this land: Draw a card."
/// When played from hand, it enters tapped. On a later turn when untapped, paying four mana from
/// other sources and tapping and sacrificing the land activates ability 1, drawing a card upon resolution.
#[test]
fn north_pole_gates_enters_tapped_and_sacrifices_to_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[north_pole_gates()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let gates = play_land(&mut engine, p0, north_pole_gates());
    assert!(is_tapped(&engine, gates), "North Pole Gates enters tapped");

    // Pass turn to untap North Pole Gates.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, gates),
        "North Pole Gates untaps in controller's untap step"
    );

    let initial_hand_len = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Tap Plains for {4} while keeping North Pole Gates untapped for its own {T} cost.
    tap_all_mana_but(&mut engine, p0, Some(north_pole_gates()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four mana floated to pay the activation cost"
    );

    activate(&mut engine, p0, north_pole_gates(), 1);

    assert!(
        on_battlefield(&engine, p0, north_pole_gates()).is_none(),
        "North Pole Gates was sacrificed to pay its activation cost"
    );
    assert!(
        in_graveyard(&engine, p0, north_pole_gates()).is_some(),
        "sacrificed land is in owner's graveyard"
    );

    pass_until(&mut engine, stack_is_empty);

    let final_hand_len = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        final_hand_len,
        initial_hand_len + 1,
        "ability 1 drew one card upon resolution"
    );
}
