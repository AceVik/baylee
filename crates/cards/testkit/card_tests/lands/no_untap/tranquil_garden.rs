//! `cards/lands/no_untap/tranquil_garden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tranquil Garden: "{T}: Add {C}." / "{T}: Add {G} or {W}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Tranquil Garden remains tapped.
#[test]
fn tranquil_garden_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(115, forest())
        .battlefield(0, &[tranquil_garden(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let garden = on_battlefield(&engine, p0, tranquil_garden()).expect("Tranquil Garden deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, tranquil_garden(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert!(is_tapped(&engine, garden));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, garden),
        "Tranquil Garden stays tapped during your next untap step"
    );
}
