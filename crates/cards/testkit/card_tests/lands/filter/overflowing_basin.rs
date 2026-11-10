//! `cards/lands/filter/overflowing_basin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Overflowing Basin prints one line and nothing else: "{1}, {T}: Add
/// {G}{U}." Its price is not the tap symbol alone, so it is not the CR 305.6
/// shortcut and no helper will ever press it: the mana ability stands in
/// `LegalActions::abilities` as `(source, 0)`, behind `can_afford`, and has to
/// be activated by hand with the {1} already floating. A Sol Ring beside it
/// pays that {1} in colourless, which is what makes the pool afterwards read
/// both printed colours at once — {G} and {U} from one activation rather than
/// a colour chosen — and the Ring's own two colourless are the arithmetic
/// that shows the {1} really was charged.
#[test]
fn overflowing_basin_charges_one_and_taps_for_green_and_blue_together() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_artifact(), overflowing_basin()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let basin =
        on_battlefield(&engine, p0, overflowing_basin()).expect("the Basin is on the table");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Ring is on the table");
    assert!(!is_tapped(&engine, basin), "a land enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before anything is tapped"
    );

    // The Ring and nothing else. Its whole price is its own {T}, so the helper
    // takes it; the Basin's "{1}, {T}" is a larger price and is left standing
    // for the activation below.
    tap_all_mana_but(&mut engine, p0, Some(overflowing_basin()));
    assert!(
        !is_tapped(&engine, basin),
        "the Basin's price is not its tap symbol alone, so the helper stepped over it"
    );
    assert!(
        is_tapped(&engine, ring),
        "and the Ring is the source that paid"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "{{C}}{{C}} off the Sol Ring, and the pool is what says so"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(basin, 0)),
        "with the {{1}} payable the one line the Basin prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, overflowing_basin(), 0);

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing was put on one"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat still holds priority, got {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the {{1}} came out of the Ring's colourless mana: two made, one left"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "{{G}} — the first of the two the card prints"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and {{U}} beside it, both colours off one activation and not a choice between them"
    );
    assert_eq!(
        pool.total(),
        3,
        "two off the Ring less the {{1}} paid, plus the two the Basin made"
    );
    assert!(is_tapped(&engine, basin), "and the Basin paid its {{T}}");
}
