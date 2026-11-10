//! `cards/lands/tapland/ancient_spring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ancient Spring` enters tapped, taps for `{{U}}`, and sacrifices for `{{T}}` to add `{{W}}{{B}}`
/// under `Coverage::Implemented`.
/// After entering tapped and untapping on the following turn, activating ability 1 without floating
/// mana sacrifices the land to the graveyard and adds one white and one black mana to an otherwise
/// empty mana pool.
#[test]
fn ancient_spring_enters_tapped_and_sacrifices_for_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2702, forest())
        .hand(0, &[ancient_spring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ancient_spring());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, ancient_spring(), 1);

    assert!(in_graveyard(&engine, p0, ancient_spring()).is_some());
    assert!(on_battlefield(&engine, p0, ancient_spring()).is_none());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 2);
}
