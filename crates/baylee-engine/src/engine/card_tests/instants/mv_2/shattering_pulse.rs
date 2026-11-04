//! `cards/instants/mv_2/shattering_pulse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shattering_pulse() -> CardIndex {
    card_index("77953b7a-813e-45f5-ad7f-ab72a79a0112")
}

/// p0 has `lands` Mountains and Shattering Pulse in hand; p1 has a Sol Ring, a
/// Gray Ogre, two Islands and a Counterspell. p0's main phase.
fn a_pulse_over_a_ring(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &vec![mountain(); lands])
        .battlefield(1, &[sol_ring(), gray_ogre(), island(), island()])
        .hand(0, &[shattering_pulse()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

#[test]
fn shattering_pulse_destroys_target_artifact() {
    let mut engine = a_pulse_over_a_ring(2);

    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, shattering_pulse());
    let creature = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&creature));
    aim_at(&mut engine, P0, ring);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, sol_ring()).is_none());
    assert!(in_graveyard(&engine, P1, sol_ring()).is_some());
    assert_not_bought_back(&engine, P0, shattering_pulse());
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home.
#[test]
fn shattering_pulse_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_pulse_over_a_ring(5);
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, P0, shattering_pulse());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ring);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{R}} and {{3}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, sol_ring()).is_some());
    assert_bought_back(&engine, P0, shattering_pulse());
}

/// Declined, it costs three less and is a Shattering Pulse that is spent.
#[test]
fn shattering_pulse_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_pulse_over_a_ring(5);
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, P0, shattering_pulse());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, ring);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, sol_ring()).is_some());
    assert_not_bought_back(&engine, P0, shattering_pulse());
}

/// Countered, it does not resolve: buyback paid, card gone, the Ring intact.
#[test]
fn shattering_pulse_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_pulse_over_a_ring(5);
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, P0, shattering_pulse());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ring);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, sol_ring()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, shattering_pulse());
}

/// With its one target gone it does not resolve either.
#[test]
fn shattering_pulse_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_pulse_over_a_ring(5);
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, P0, shattering_pulse());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ring);
    bury(&mut engine, &[ring]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, shattering_pulse());
}
