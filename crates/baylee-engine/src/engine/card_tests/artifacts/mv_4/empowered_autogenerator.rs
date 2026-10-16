//! `cards/artifacts/mv_4/empowered_autogenerator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn empowered_autogenerator() -> CardIndex {
    card_index("6832f9e2-294a-44a2-8af9-eb16ccfdfc36")
}

/// "This artifact enters tapped."
#[test]
fn the_autogenerator_enters_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[empowered_autogenerator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, empowered_autogenerator());
    pass_until(&mut engine, stack_is_empty);
    let card = on_battlefield(&engine, p0, empowered_autogenerator()).expect("resolved");
    assert!(is_tapped(&engine, card));
}

/// "{T}: Put a charge counter on this artifact. Add X mana of any one color,
/// where X is the number of charge counters on this artifact." The counter
/// goes on first, so the first tap already makes one mana and the second
/// makes two, all of the one chosen color.
#[test]
fn each_tap_adds_a_counter_and_then_that_many_mana_of_one_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[empowered_autogenerator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = on_battlefield(&engine, p0, empowered_autogenerator()).unwrap();
    assert_eq!(counters_on(&engine, card, CounterKind::Charge), 0);

    for (round, total) in [(1u16, 1u64), (2, 3)] {
        activate(&mut engine, p0, empowered_autogenerator(), 0);
        while matches!(engine.pending(), Pending::ChooseColor { .. }) {
            engine
                .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
                .expect("a listed color");
        }
        assert_eq!(counters_on(&engine, card, CounterKind::Charge), round);
        assert_eq!(
            u64::from(engine.state().players[0].mana_pool.available(ManaColor::Red)),
            total,
            "after tap {round} the pool holds the running total of red"
        );
        assert_eq!(engine.state().players[0].mana_pool.total(), total);
        engine.dev_state_mut(p0).unwrap().set_tapped(card, false);
        engine.refresh_offer();
    }
}
