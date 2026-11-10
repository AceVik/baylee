//! `cards/creatures/mv_8/baleful_force.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn baleful_force() -> CardIndex {
    card_index("f26fa9e4-f528-4e22-8f7c-329fc3b633e4")
}

/// Baleful Force is a 7/7 for {5}{B}{B}{B}.
#[test]
fn baleful_force_is_cast_for_eight_as_a_seven_seven() {
    let mut engine = at_main(&[swamp(); 8], &[baleful_force()]);
    let force = cast_and_settle(&mut engine, baleful_force());
    assert_eq!(pt(&engine, force), (7, 7));
}

/// "At the beginning of each upkeep, you draw a card and you lose 1 life":
/// on the opponent's upkeep the controller draws one card and loses 1; over
/// the full turn cycle that comes back to the controller's own main phase it
/// has drawn three (the opponent's upkeep, its own upkeep and its own draw
/// step) and lost 2.
#[test]
fn baleful_force_draws_and_costs_a_life_on_every_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_main(&[baleful_force()], &[]);
    let (life, hand) = (life_of(&engine, 0), hand_size(&engine, 0));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(life_of(&engine, 0), life - 1, "the opponent's upkeep");
    assert_eq!(hand_size(&engine, 0), hand + 1, "and one card for it");

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(life_of(&engine, 0), life - 2, "its own upkeep too");
    assert_eq!(
        hand_size(&engine, 0),
        hand + 3,
        "two trigger draws and the draw step"
    );
}
