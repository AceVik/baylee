//! `cards/lands/tapland/bog_wreckage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bog Wreckage` enters tapped, taps for `{{B}}`, and sacrifices for `{{T}}` to add one mana
/// of any color under `Coverage::Implemented`.
/// After entering tapped and untapping on the following turn, activating ability 1 without floating
/// mana prompts for a color choice via `Pending::ChooseColor`.
/// Choosing red mana sacrifices the land to the graveyard and adds exactly one red mana to
/// an otherwise empty mana pool.
#[test]
fn bog_wreckage_enters_tapped_and_sacrifices_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2705, forest()).hand(0, &[bog_wreckage()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, bog_wreckage());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, bog_wreckage(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert!(in_graveyard(&engine, p0, bog_wreckage()).is_some());
    assert!(on_battlefield(&engine, p0, bog_wreckage()).is_none());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.total(), 1);
}
