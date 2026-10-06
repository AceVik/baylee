//! `cards/lands/tapland/dwarven_ruins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dwarven Ruins` enters tapped, taps for `{{R}}`, and sacrifices for `{{T}}` to add `{{R}}{{R}}`
/// under `Coverage::Implemented`.
/// After entering tapped and untapping on the following turn, activating ability 1 without floating
/// mana sacrifices the land to the graveyard and adds two red mana to an otherwise empty mana pool.
#[test]
fn dwarven_ruins_enters_tapped_and_sacrifices_for_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2711, forest())
        .hand(0, &[dwarven_ruins()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, dwarven_ruins());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, dwarven_ruins(), 1);

    assert!(in_graveyard(&engine, p0, dwarven_ruins()).is_some());
    assert!(on_battlefield(&engine, p0, dwarven_ruins()).is_none());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 2);
    assert_eq!(pool.total(), 2);
}
