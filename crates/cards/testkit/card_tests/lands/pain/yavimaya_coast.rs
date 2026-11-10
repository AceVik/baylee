//! `cards/lands/pain/yavimaya_coast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Yavimaya Coast` taps for `{{C}}` or taps for `{{G}}` or `{{U}}` while dealing 1 damage
/// to its controller under `Coverage::Implemented`.
/// Activating ability 1 prompts for a color choice, adds the chosen colored mana, and reduces
/// its controller's life total from 20 to 19.
#[test]
fn yavimaya_coast_adds_colored_mana_and_deals_damage_to_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1918, forest())
        .hand(0, &[yavimaya_coast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, yavimaya_coast());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, yavimaya_coast(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::Blue));

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
    assert_eq!(engine.state().players[0].life, 19);
}
