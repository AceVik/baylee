//! `cards/lands/no_untap/rootwater_depths.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rootwater Depths: "{T}: Add {C}." / "{T}: Add {U} or {B}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Rootwater Depths remains tapped.
#[test]
fn rootwater_depths_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[rootwater_depths(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let depths =
        on_battlefield(&engine, p0, rootwater_depths()).expect("Rootwater Depths deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, rootwater_depths(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, depths));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, depths),
        "Rootwater Depths stays tapped during your next untap step"
    );
}
