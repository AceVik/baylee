//! `cards/lands/planets/uthros_titanic_godcore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Uthros, Titanic Godcore prints `This land enters tapped.`, `{{T}}: Add {{U}}.`,
/// Station, and `12+ | {{U}}, {{T}}: Add {{U}} for each artifact you control.`
///
/// Under `Coverage::Partial`, Station is omitted because the engine DSL cannot
/// read the tapped creature's power from a cost. This test plays the land, confirms
/// that it enters tapped, advances to the next turn to untap it, checks that
/// ability 1 is withheld without twelve charge counters, and taps ability 0 for `{U}`.
#[test]
fn uthros_titanic_godcore_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[uthros_titanic_godcore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, uthros_titanic_godcore());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        1,
        "only ability 0 is offered because charge counter threshold is not met"
    );

    activate(&mut engine, p0, uthros_titanic_godcore(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, land));
}
