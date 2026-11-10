//! `cards/lands/pain/underground_river.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Underground River` taps for `{{C}}` or taps for `{{U}}` or `{{B}}` while dealing 1 damage
/// to its controller under `Coverage::Implemented`.
/// Activating ability 1 prompts for a color choice, adds the chosen colored mana, and reduces
/// its controller's life total by 1.
#[test]
fn underground_river_adds_colored_mana_and_deals_damage_to_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1917, forest())
        .hand(0, &[underground_river()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, underground_river());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, underground_river(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert!(is_tapped(&engine, land));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert_eq!(engine.state().players[0].life, 19);
}
