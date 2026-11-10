//! `cards/lands/grove_of_the_burnwillows.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Grove of the Burnwillows` taps for `{{C}}` or taps for `{{R}}` or `{{G}}` while giving each
/// opponent 1 life under `Coverage::Implemented`.
/// Activating ability 1 prompts for a color choice, adds the chosen colored mana, and increases
/// the opponent's life total from 20 to 21.
#[test]
fn grove_of_the_burnwillows_adds_colored_mana_and_gives_opponent_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1916, forest())
        .hand(0, &[grove_of_the_burnwillows()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, grove_of_the_burnwillows());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, grove_of_the_burnwillows(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::Green));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    assert!(is_tapped(&engine, land));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(engine.state().players[1].life, 21);
}
