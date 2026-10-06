//! `cards/lands/tendo_ice_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tendo Ice Bridge: "This land enters **with** a charge counter on it."
///
/// The same sentence as the Vivid cycle's without the word "tapped", and it
/// is a separate card rather than a parameter because the difference is the
/// whole card: Tendo is usable the turn it is played, so the counter is
/// spent on that turn's colour instead of next turn's.
#[test]
fn tendo_ice_bridge_enters_untapped_and_spends_its_one_counter_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(543, forest())
        .hand(0, &[tendo_ice_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, tendo_ice_bridge());
    assert!(
        !entered_tapped(&engine, land),
        "the printing does not say tapped"
    );
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        1,
        "and it says one counter"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the counter arrived with the land and can be spent at once");
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("any colour is a choice: {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("a colour the engine offered");

    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "the counter is gone"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "and green came out of a land that otherwise makes {{C}}"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)) && !legal.abilities.contains(&(land, 1)),
        "untapped again, and only the line that costs no counter is offered: \
         {:?}",
        legal.abilities
    );
}
