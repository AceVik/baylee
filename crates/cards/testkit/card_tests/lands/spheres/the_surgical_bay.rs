//! `cards/lands/spheres/the_surgical_bay.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `The Surgical Bay` enters tapped, taps for `{{U}}`, and sacrifices for `{{1}}{{U}}, {{T}}`
/// to draw a card under `Coverage::Implemented`.
/// After entering tapped, walking to the controller's next turn allows activating ability 1
/// with floating mana from two Islands, sacrificing the land and drawing a card.
#[test]
fn the_surgical_bay_enters_tapped_and_sacrifices_to_draw() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2335, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[the_surgical_bay()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_surgical_bay());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(the_surgical_bay()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    let before_lib = library_size(&engine, p0);
    activate(&mut engine, p0, the_surgical_bay(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, the_surgical_bay()).is_some());
    assert!(on_battlefield(&engine, p0, the_surgical_bay()).is_none());
    assert_eq!(library_size(&engine, p0), before_lib - 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
