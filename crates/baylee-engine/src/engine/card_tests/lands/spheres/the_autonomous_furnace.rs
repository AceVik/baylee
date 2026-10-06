//! `cards/lands/spheres/the_autonomous_furnace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `The Autonomous Furnace` enters tapped, taps for `{{R}}`, and sacrifices for `{{1}}{{R}}, {{T}}`
/// to draw a card under `Coverage::Implemented`.
/// After entering tapped, walking to the controller's next turn allows activating ability 1
/// with floating mana from two Mountains, sacrificing the land and drawing a card.
#[test]
fn the_autonomous_furnace_enters_tapped_and_sacrifices_to_draw() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2331, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[the_autonomous_furnace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_autonomous_furnace());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(the_autonomous_furnace()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    let before_lib = library_size(&engine, p0);
    activate(&mut engine, p0, the_autonomous_furnace(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, the_autonomous_furnace()).is_some());
    assert!(on_battlefield(&engine, p0, the_autonomous_furnace()).is_none());
    assert_eq!(library_size(&engine, p0), before_lib - 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
