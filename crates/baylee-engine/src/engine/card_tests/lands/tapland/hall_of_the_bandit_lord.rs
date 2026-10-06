//! `cards/lands/tapland/hall_of_the_bandit_lord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hall of the Bandit Lord: "Hall of the Bandit Lord enters tapped." / "{T}, Pay 3 life: Add {C}. If that mana is spent on a creature spell, it gains haste."
/// Under `Coverage::Partial`, the haste grant rider on spent mana is omitted.
/// Playing the land verifies it enters tapped; upon untapping next turn, paying 3 life and tapping adds {C} to the pool.
#[test]
fn hall_of_the_bandit_lord_enters_tapped_and_costs_life_for_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(132, forest())
        .hand(0, &[hall_of_the_bandit_lord()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = play_land(&mut engine, p0, hall_of_the_bandit_lord());
    assert!(entered_tapped(&engine, hall), "enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, hall), "untaps on next turn");

    let life_before = engine.state().players[0].life;
    activate(&mut engine, p0, hall_of_the_bandit_lord(), 0);

    assert_eq!(engine.state().players[0].life, life_before - 3);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hall));
}
