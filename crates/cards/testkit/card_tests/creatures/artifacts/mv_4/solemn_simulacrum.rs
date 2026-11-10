//! `cards/creatures/artifacts/mv_4/solemn_simulacrum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Solemn Simulacrum: "When this creature enters, you may search your
/// library for a basic land card, put that card onto the battlefield tapped,
/// then shuffle. When this creature dies, you may draw a card."
#[test]
fn solemn_simulacrum_fetches_a_tapped_basic_and_draws_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4501, forest())
        .battlefield(0, &[island(), island(), island(), island(), mountain()])
        .hand(0, &[solemn_simulacrum(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lib_before = library_size(&engine, p0);

    // Four Islands pay {4}; the Mountain is held back for the Bolt.
    tap_all_mana_but(&mut engine, p0, Some(mountain()));
    cast_with_floating(&mut engine, p0, solemn_simulacrum());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the search asks for its land")
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the land came from the search");
    pass_until(&mut engine, stack_is_empty);
    let fetched = all_on_battlefield(&engine, p0, forest());
    assert_eq!(fetched.len(), 1, "a basic land arrived");
    assert!(
        is_tapped(&engine, fetched[0]),
        "\"put that card onto the battlefield tapped\""
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "taken from the library"
    );

    // Now the Bolt, aimed at the Simulacrum: "when this creature dies".
    let golem = on_battlefield(&engine, p0, solemn_simulacrum()).expect("the Golem");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![golem],
            },
        )
        .expect("the Bolt aims at the Golem");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, solemn_simulacrum()).is_some(),
        "it died"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 1,
        "the Bolt left the hand and the death drew one card"
    );
    assert_eq!(library_size(&engine, p0), lib_before - 1);
}
