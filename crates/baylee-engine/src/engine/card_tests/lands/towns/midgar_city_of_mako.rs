//! `cards/lands/towns/midgar_city_of_mako.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Midgar, City of Mako is a land with an Adventure on its back, and the file
/// is `Coverage::Partial`: the land half is written and Reactor Raid's
/// reflexive "If you do, draw two cards" is not. So the two printed sentences
/// the card *does* carry are what a board can hold it to — "This land enters
/// tapped" and "{T}: Add {B}" — and both are only worth asserting together,
/// because a land that entered untapped would also make its mana and read as
/// fine.
///
/// The Adventure half is checked by its absence rather than left unsaid: the
/// card is played as a land out of hand, and nothing about that offer is a
/// spell. A second Town is deliberately *not* on the board — an enters-tapped
/// clause that was really a conditional one would still be tapped here, and
/// this land prints no condition to be wrong about.
#[test]
fn midgar_city_of_mako_enters_tapped_and_makes_the_one_colour_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[midgar_city_of_mako()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, midgar_city_of_mako());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is unconditional on this card"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "the face played out of hand is the land face and not the Adventure"
    );

    // A tapped land makes no mana, so the turn has to come round before the
    // second sentence can be read at all.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && !is_tapped(e, land)
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before the ability is activated"
    );
    activate(&mut engine, p0, midgar_city_of_mako(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{T}}: Add {{B}} — one black and not a choice of five"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and one mana in total: a Town is not a dual"
    );
    assert!(is_tapped(&engine, land), "its own tap symbol was the cost");
}
