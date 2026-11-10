//! `cards/lands/tapland/opulent_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Opulent Palace` enters tapped and taps for `{{B}}`, `{{G}}`, or `{{U}}` under `Coverage::Implemented`.
/// After entering tapped, walking across the opponent's turn to the controller's next main phase
/// untaps the land, allowing its mana ability to offer all three colors.
#[test]
fn opulent_palace_enters_tapped_and_produces_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3304, forest())
        .hand(0, &[opulent_palace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, opulent_palace());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, opulent_palace(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 3);
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Black), 0);
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
