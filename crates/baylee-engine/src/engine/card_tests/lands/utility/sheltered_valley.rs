//! `cards/lands/utility/sheltered_valley.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sheltered Valley` prints `If this land would enter, instead sacrifice each other permanent named Sheltered Valley you control, then put this land onto the battlefield.`, `At the beginning of your upkeep, if you control three or fewer lands, you gain 1 life.`, and `{{T}}: Add {{C}}.`
///
/// Under `Coverage::Partial`, the namesake sacrifice replacement is unsupported: a second copy played from hand enters beside the first. That half is pinned, and moves when the replacement is written.
/// Both copies tap for one colorless mana each.
#[test]
fn sheltered_valley_enters_beside_its_namesake_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sheltered_valley()])
        .hand(0, &[sheltered_valley()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let second = play_land(&mut engine, p0, sheltered_valley());
    assert!(!entered_tapped(&engine, second));
    assert_eq!(
        all_on_battlefield(&engine, p0, sheltered_valley()).len(),
        2,
        "pinned: no replacement sacrifices the first copy"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 2, "both Sheltered Valleys tap for mana");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 2);
    assert_eq!(pool.total(), 2);
}

/// Sheltered Valley: "At the beginning of your upkeep, if you control three
/// or fewer lands, you gain 1 life." Measured over one full turn cycle
/// (the opponent's turn and then this seat's upkeep), on three lands and on
/// four. The four-land board is the half that says the count is bounded from
/// above, and the opponent's upkeep in between is the half that says `your`.
#[test]
fn sheltered_valley_gains_a_life_in_your_upkeep_on_three_lands_and_none_on_four() {
    let p0 = PlayerId::new(0);
    let gained_over_a_turn_cycle = |lands: &[CardIndex]| {
        let mut engine = Duel::new(SEED, forest()).battlefield(0, lands).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let before = engine.state().players[0].life;
        cross_into_the_next_own_main(&mut engine, p0);
        engine.state().players[0].life - before
    };

    assert_eq!(
        gained_over_a_turn_cycle(&[sheltered_valley(), forest(), forest()]),
        1,
        "three lands: one upkeep of this seat's, one life"
    );
    assert_eq!(
        gained_over_a_turn_cycle(&[sheltered_valley(), forest(), forest(), forest()]),
        0,
        "four lands is one too many"
    );
}
