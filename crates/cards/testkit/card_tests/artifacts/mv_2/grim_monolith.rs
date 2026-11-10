//! `cards/artifacts/mv_2/grim_monolith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grim Monolith ({2}): the same held-down artifact for one mana less, and
/// the untap costs `{4}` where Basalt Monolith's costs `{3}`.
///
/// Written because the difference is the *only* thing a second copy of the
/// rule is worth testing. The three colourless the artifact just made are
/// not four, so the way out is not even offered — an ability nobody is
/// offered is how every cost this engine cannot pay is refused, and asking
/// the offer is what tells a `{4}` in the card file from a `{3}`.
#[test]
fn grim_monolith_asks_four_for_the_untap_its_untap_step_will_not_give() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9902, forest())
        .battlefield(0, &[grim_monolith(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monolith =
        on_battlefield(&engine, p0, grim_monolith()).expect("the Monolith is on the table");
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(monolith, 2))
        )
    };

    activate(&mut engine, p0, grim_monolith(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "three colourless off the artifact"
    );
    assert!(
        !offered(&engine),
        "and three is not four: the untap is not offered, which is how a \
         cost this board cannot pay is refused"
    );

    tap_all_mana(&mut engine, p0);
    assert!(offered(&engine), "with the Forests it is six, and it is");

    assert!(is_tapped(&engine, monolith), "still tapped for now");
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "back to its controller");
    assert!(
        is_tapped(&engine, monolith),
        "and the untap step left it alone, the same rule Basalt Monolith \
         prints (CR 502.3)"
    );
}
