//! `cards/lands/tapland/irrigation_ditch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Irrigation Ditch` enters tapped, taps for `{{W}}`, and sacrifices for `{{T}}` to add `{{G}}{{U}}`
/// under `Coverage::Implemented`.
/// After entering tapped and untapping on the following turn, activating ability 1 without floating
/// mana sacrifices the land to the graveyard and adds one green and one blue mana to an otherwise
/// empty mana pool.
#[test]
fn irrigation_ditch_enters_tapped_and_sacrifices_for_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2728, forest())
        .hand(0, &[irrigation_ditch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, irrigation_ditch());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, irrigation_ditch(), 1);

    assert!(in_graveyard(&engine, p0, irrigation_ditch()).is_some());
    assert!(on_battlefield(&engine, p0, irrigation_ditch()).is_none());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 2);
}
