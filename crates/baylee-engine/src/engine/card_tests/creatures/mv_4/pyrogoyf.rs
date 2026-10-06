//! `cards/creatures/mv_4/pyrogoyf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// CR 604.3: a characteristic-defining ability works in every zone, so
/// "toughness 2 or less" reads the number the card defines, not its printed
/// `*` as 0. In the library Ashaya is a 3/3 beside three Plains and is never
/// offered; Pyrogoyf is a 0/1 over empty graveyards and is, and a 2/3 once
/// the graveyards hold a land card and a creature card, and is not. The
/// Elves are on every menu, so an empty one proves nothing.
///
/// Before, a static defining P/T was registered only on the battlefield,
/// and both cards were offered at toughness 0 every time.
#[test]
fn recruiter_of_the_guard_reads_a_toughness_its_card_defines_in_the_library() {
    let empty = recruiter_menu_over_defined_bodies(false);
    assert!(empty.contains(&llanowar_elves()), "{empty:?}");
    assert!(
        !empty.contains(&ashaya_soul_of_the_wild()),
        "Ashaya is a 3/3 in the library: {empty:?}"
    );
    assert!(
        empty.contains(&pyrogoyf()),
        "Pyrogoyf over empty graveyards is a 0/1: {empty:?}"
    );

    let two = recruiter_menu_over_defined_bodies(true);
    assert!(two.contains(&llanowar_elves()), "{two:?}");
    assert!(!two.contains(&ashaya_soul_of_the_wild()), "{two:?}");
    assert!(
        !two.contains(&pyrogoyf()),
        "a land card and a creature card in the graveyards make Pyrogoyf a \
         2/3: {two:?}"
    );
}

/// "Pyrogoyf's power is equal to the number of card types among cards in
/// all graveyards and its toughness is equal to that number plus 1." Empty
/// graveyards: 0/1. A creature card milled into the opponent's: 1/2 — a
/// card that went from a library to a graveyard, with no permanent moving,
/// still grows it. A second creature card adds no type. A land card joins:
/// 2/3.
#[test]
fn pyrogoyf_counts_the_card_types_in_every_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, steadfast_guard())
        .battlefield(0, &[pyrogoyf(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let goyf = on_battlefield(&engine, p0, pyrogoyf()).unwrap();
    assert_eq!(pt(&engine, goyf), (0, 1), "no card in any graveyard");

    // The harness mills outside the engine's loop, so the refresh the
    // engine runs before its next priority grant is run by hand; what it
    // proves is that a mill made the projection stale at all.
    let refresh = |engine: &mut Engine<RegistryLookup>| {
        engine
            .dev_state_mut(p0)
            .expect("the harness may look")
            .refresh_characteristics();
    };
    seed_graveyard(&mut engine, p1, 1);
    refresh(&mut engine);
    assert_eq!(
        pt(&engine, goyf),
        (1, 2),
        "a creature card, in the opponent's graveyard"
    );
    seed_graveyard(&mut engine, p0, 1);
    refresh(&mut engine);
    assert_eq!(
        pt(&engine, goyf),
        (1, 2),
        "a second creature card is no second type"
    );

    let land = on_battlefield(&engine, p0, mountain()).unwrap();
    bury(&mut engine, &[land]);
    refresh(&mut engine);
    assert_eq!(pt(&engine, goyf), (2, 3), "creature and land");
}

/// The Pyrogoyf leaves before its trigger resolves: it deals damage equal to
/// its power as it last existed on the battlefield (CR 608.2h) — 2, not the
/// 0 a card in a graveyard prints, and not the new count its own arrival in
/// the graveyard would make.
#[test]
fn pyrogoyf_that_has_left_deals_its_last_known_power() {
    let (mut engine, goyf) = a_pyrogoyf_aimed_at_the_opponent();
    bury(&mut engine, &[goyf]);
    assert!(on_battlefield(&engine, PlayerId::new(0), pyrogoyf()).is_none());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18, "2, as it last existed");
}
