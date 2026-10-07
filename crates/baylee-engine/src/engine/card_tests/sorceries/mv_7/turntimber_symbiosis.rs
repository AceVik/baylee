//! `cards/sorceries/mv_7/turntimber_symbiosis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Turntimber Symbiosis` // `Turntimber, Serpentine Wood` (`Coverage::Partial`):
/// "Look at the top seven cards of your library. You may put a creature card from among them onto the
/// battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters
/// on it. Put the rest on the bottom of your library in a random order. // As this land enters, you may
/// pay 3 life. If you don't, it enters tapped. `{{T}}`: Add `{{G}}`."
///
/// Under `Coverage::Partial`, the front-face creature search is omitted, while the back-face land
/// (`Turntimber, Serpentine Wood`) is implemented in full. The test plays the back face as a land,
/// declines paying 3 life so that it enters tapped without life loss, advances to the next turn so it
/// untaps, and activates its mana ability to add `{{G}}`.
#[test]
fn turntimber_serpentine_wood_enters_tapped_on_declined_life_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, land) =
        play_land_face(turntimber_symbiosis(), 1).expect("plays as Turntimber, Serpentine Wood");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    }

    assert!(
        is_tapped(&engine, land),
        "declined paying 3 life, so it entered tapped"
    );
    assert_eq!(engine.state().players[0].life, 20, "life remains at 20");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");

    assert!(is_tapped(&engine, land), "tapped for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green mana added to pool"
    );
}
