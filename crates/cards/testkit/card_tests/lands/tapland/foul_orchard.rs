//! `cards/lands/tapland/foul_orchard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Foul Orchard` enters tapped and taps for `{{B}}` or `{{G}}` under `Coverage::Implemented`.
/// After entering tapped, walking across the opponent's turn to the controller's next main phase
/// untaps the land, allowing its mana ability to be activated for a chosen color.
#[test]
fn foul_orchard_enters_tapped_and_produces_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2715, forest()).hand(0, &[foul_orchard()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, foul_orchard());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, foul_orchard(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Black), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
