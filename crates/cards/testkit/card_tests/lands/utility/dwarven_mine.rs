//! `cards/lands/utility/dwarven_mine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Eldraine trio: "This land enters tapped unless you control three or
/// more other \[basics of its own type\]. When this land enters untapped, …".
///
/// Two sentences and they are wired to each other, which is what this plays.
/// The count is `at_least: 3` and the "other" is the engine's, so a Grange
/// standing beside two Plains is the third Plains and still not three
/// *other* ones. And the second sentence is gated on the first: a land that
/// came down tapped triggers nothing at all, which is why the token half is
/// asserted on both sides rather than only where it appears.
///
/// Dwarven Mine and Gingerbread Cabin carry the half that is observable
/// without a target — a Dwarf and a Food — while Idyllic Grange's counter
/// wants a creature to go on, so it is played here for the tapped half.
#[test]
fn an_eldraine_land_wants_three_others_of_its_own_type() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(214, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[dwarven_mine(), idyllic_grange()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = play_land(&mut engine, p0, dwarven_mine());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        entered_tapped(&engine, mine),
        "two Mountains are not three other Mountains"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "a land that entered tapped makes no Dwarf"
    );
}

/// The other side of the same sentence, one Mountain further along.
#[test]
fn an_eldraine_land_that_enters_untapped_makes_its_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(215, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[dwarven_mine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = play_land(&mut engine, p0, dwarven_mine());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !entered_tapped(&engine, mine),
        "three Mountains are three other Mountains"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "the Mine makes one Dwarf on the way in"
    );
}
