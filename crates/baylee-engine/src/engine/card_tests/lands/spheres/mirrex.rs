//! `cards/lands/spheres/mirrex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mirrex: "{T}: Add {C}." / "{T}: Add one mana of any color. Activate only if this land entered this turn." / "{3}, {T}: Create a 1/1 colorless Phyrexian Mite..."
/// Under `Coverage::Partial`, the Mite token ability is omitted; the entered-this-turn any-color
/// ability is written and played in `mirrex_enters_untapped_adds_any_color_only_on_entry_turn`.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn mirrex_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(327, forest()).hand(0, &[mirrex()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrex());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, mirrex(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Mirrex prints `{{T}}: Add {{C}}.`,
/// `{{T}}: Add one mana of any color. Activate only if this land entered this turn.`, and
/// `{{3}}, {{T}}: Create a 1/1 colorless Phyrexian Mite artifact creature token...`
/// Under `Coverage::Partial`, token creation is unsupported.
/// This test plays Mirrex untapped, verifies that floating `{{3}}` with `tap_mana_except` does
/// not offer the unsupported token ability, activates ability 1 to add one mana of any chosen color
/// on the turn it entered, and advances to the next turn where ability 1 is no longer offered and
/// ability 0 taps for colorless mana.
#[test]
fn mirrex_enters_untapped_adds_any_color_only_on_entry_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[mirrex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mirrex());
    assert!(!entered_tapped(&engine, land), "Mirrex enters untapped");

    // Float {3} mana keeping Mirrex untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three green mana floating to afford potential costs"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };

    // The unsupported token creation ability is not offered even with {3} floating.
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 2),
        "token creation ability is unsupported under `Coverage::Partial` even with {{3}} floating"
    );

    // Both ability 0 ({C}) and ability 1 (any color on entry turn) are available.
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 0),
        "ability 0 is offered"
    );
    assert!(
        legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 1),
        "ability 1 is offered on the turn Mirrex entered"
    );

    // Activate ability 1 to produce blue mana.
    activate(&mut engine, p0, mirrex(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "offers all 5 colors");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choosing blue mana is legal");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "adds one blue mana"
    );
    assert!(is_tapped(&engine, land), "Mirrex tapped for ability 1");

    // On the following turn, Mirrex did not enter this turn and ability 1 is withheld.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "Mirrex untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 1),
        "ability 1 is withheld when Mirrex did not enter this turn"
    );

    // Ability 0 taps for colorless mana.
    activate(&mut engine, p0, mirrex(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "ability 0 adds {{C}}"
    );
    assert!(is_tapped(&engine, land));
}
