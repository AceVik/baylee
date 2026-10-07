//! `cards/lands/towns/jidoor_aristocratic_capital.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jidoor is a Town — a land on the front of a card whose back is an
/// Adventure — and the half worth playing is that the land drop is the land.
/// "Overture" is a {4}{U}{U} sorcery on the other face, so a card offering
/// its back out of hand would be a six-mana spell anybody could play for
/// free as a land, which is exactly the fault twenty-one cards in this pool
/// had this morning.
///
/// The file is `Coverage::Partial` for Overture's own text — "mills half
/// their library, rounded down" has no `Amount` that halves — and the
/// Adventure is therefore checked by its absence rather than cast.
#[test]
fn jidoor_is_played_as_the_town_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[jidoor_aristocratic_capital()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, jidoor_aristocratic_capital());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is unconditional"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "what arrived is the Town and not the sorcery on its back"
    );
    assert!(
        !types(&engine, land).contains(TypeSet::SORCERY),
        "and the Adventure face stayed on the other side of the card"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    activate(&mut engine, p0, jidoor_aristocratic_capital(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "{{T}}: Add {{U}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one blue and nothing else"
    );
}
