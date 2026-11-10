//! `cards/creatures/mv_8/celestial_force.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn celestial_force() -> CardIndex {
    card_index("8add6996-6c5c-4aa2-93d4-a39efcfd22b2")
}

/// Celestial Force is a 7/7 for {5}{W}{W}{W}.
#[test]
fn celestial_force_is_cast_for_eight_as_a_seven_seven() {
    let mut engine = at_main(&[plains(); 8], &[celestial_force()]);
    let force = cast_and_settle(&mut engine, celestial_force());
    assert_eq!(pt(&engine, force), (7, 7));
}

/// "At the beginning of each upkeep, you gain 3 life": its controller gains
/// 3 on the opponent's upkeep and 3 more on its own, 6 over the turn cycle;
/// the opponent gains nothing.
#[test]
fn celestial_force_gains_three_on_every_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_main(&[celestial_force()], &[]);
    let (mine, theirs) = (life_of(&engine, 0), life_of(&engine, 1));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(life_of(&engine, 0), mine + 3, "the opponent's upkeep");

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(life_of(&engine, 0), mine + 6, "its own upkeep as well");
    assert_eq!(life_of(&engine, 1), theirs, "the opponent gains nothing");
}
