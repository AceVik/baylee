//! `cards/lands/refuge/gates_of_istfell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Gates of Istfell` enters tapped, taps for `{{W}}`, and sacrifices for `{{2}}{{W}}{{U}}{{U}}, {{T}}`
/// to gain 2 life and draw two cards under `Coverage::Implemented`.
/// After entering tapped and untapping on the subsequent turn, floating the activation mana allows it
/// to be sacrificed, increasing life to 22 and drawing two cards.
#[test]
fn gates_of_istfell_enters_tapped_and_sacrifices_to_gain_life_and_draw() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1919, forest())
        .battlefield(0, &[plains(), plains(), island(), island(), forest()])
        .hand(0, &[gates_of_istfell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, gates_of_istfell());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    assert!(!is_tapped(&engine, land));

    let library_before = library_size(&engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(gates_of_istfell()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);

    activate(&mut engine, p0, gates_of_istfell(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, gates_of_istfell()).is_some());
    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(library_size(&engine, p0), library_before - 2);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
