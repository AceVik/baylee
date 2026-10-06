//! `cards/lands/utility/argoth_sanctum_of_nature.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Argoth, Sanctum of Nature prints `This land enters tapped unless you control a legendary
/// green creature.`, `{{T}}: Add {{G}}.`, `{{2}}{{G}}{{G}}, {{T}}: Create a 2/2 green Bear creature
/// token, then mill three cards. Activate only as a sorcery.`, and `(Melds with Titania, Voice of Gaea.)`
///
/// Under `Coverage::Partial`, meld is omitted. Without a legendary green creature,
/// playing this land enters tapped. After advancing to the next turn, it untaps and taps for one
/// green mana; `argoth_sanctum_of_nature_makes_a_bear_and_mills_three` plays the Bear ability.
#[test]
fn argoth_sanctum_of_nature_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[argoth_sanctum_of_nature()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, argoth_sanctum_of_nature());
    assert!(
        entered_tapped(&engine, land),
        "without a legendary green creature, Argoth enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    // Float {4} green mana while keeping Argoth untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, argoth_sanctum_of_nature(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 5);
    assert_eq!(pool.total(), 5);
    assert!(is_tapped(&engine, land));
}

/// Argoth's `{{2}}{{G}}{{G}}, {{T}}: Create a 2/2 green Bear creature token, then
/// mill three cards. Activate only as a sorcery.` One Bear, and exactly three
/// cards from the library to the graveyard.
#[test]
fn argoth_sanctum_of_nature_makes_a_bear_and_mills_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argoth_sanctum_of_nature(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, argoth_sanctum_of_nature()).expect("Argoth deployed");
    tap_mana_except(&mut engine, p0, land);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    activate(&mut engine, p0, argoth_sanctum_of_nature(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, land));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Bear");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library - 3
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard + 3
    );
}
