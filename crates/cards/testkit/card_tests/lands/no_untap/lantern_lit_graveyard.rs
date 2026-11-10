//! `cards/lands/no_untap/lantern_lit_graveyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lantern-Lit Graveyard: "{T}: Add {C}." / "{T}: Add {B} or {R}. This land doesn't untap during your next untap step."
/// Activating the second ability produces the chosen colored mana and creates a suppression effect.
/// Across the opponent's turn and into the next turn, a tapped Forest untaps while Lantern-Lit Graveyard remains tapped.
#[test]
fn lantern_lit_graveyard_produces_colored_mana_and_does_not_untap_next_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(110, forest())
        .battlefield(0, &[lantern_lit_graveyard(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let graveyard =
        on_battlefield(&engine, p0, lantern_lit_graveyard()).expect("Graveyard deployed");
    let f = on_battlefield(&engine, p0, forest()).expect("Forest deployed");

    activate(&mut engine, p0, lantern_lit_graveyard(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert!(is_tapped(&engine, graveyard));

    tap_all_mana(&mut engine, p0);
    assert!(is_tapped(&engine, f));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, f), "Forest untapped as normal");
    assert!(
        is_tapped(&engine, graveyard),
        "Lantern-Lit Graveyard stays tapped during your next untap step"
    );
}
