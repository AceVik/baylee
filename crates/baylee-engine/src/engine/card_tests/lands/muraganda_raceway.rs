//! `cards/lands/muraganda_raceway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Muraganda Raceway prints `Start your engines!`, `{{T}}: Add {{C}}`, and
/// `Max speed — {{T}}: Add {{C}}{{C}}.`
///
/// Under `Coverage::Partial`, speed mechanics and the max speed bonus ability
/// are omitted because speed is not modeled in the engine. This test plays the
/// land, verifies that it offers only its single unconditional mana ability,
/// and taps it for `{C}`.
#[test]
fn muraganda_raceway_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[muraganda_raceway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, muraganda_raceway());
    assert!(!entered_tapped(&engine, land), "enters untapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        1,
        "offers only the basic mana ability; max speed is not modeled"
    );

    activate(&mut engine, p0, muraganda_raceway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one colorless and nothing else"
    );
}
