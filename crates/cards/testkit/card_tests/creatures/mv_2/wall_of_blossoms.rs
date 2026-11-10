//! `cards/creatures/mv_2/wall_of_blossoms.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Blossoms prints a {1}{G} 0/4 Plant Wall with "Defender" and "When
/// this creature enters, draw a card", and the board reads each printed fact
/// as a different question: two Forests pay the {1}{G}, the permanent arrives
/// as the printed 0/4 with the printed keyword still on it, and the library is
/// exactly one shorter once the enters-trigger has resolved. The hand of the
/// same size is the other half of that claim — the Wall left the hand and the
/// draw put a single card back, so a hand one card *shorter* would be the same
/// library count with no draw at all. The Wall is cast and never seated: a
/// `starting_battlefield` permanent never runs its printed entry ability, so a
/// test built that way would be asserting a trigger the engine was never asked
/// to fire.
#[test]
fn wall_of_blossoms_arrives_as_a_defending_wall_and_draws_for_coming_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[wall_of_blossoms()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Both halves of the claim are read across the cast: the library the entry
    // trigger draws from, and the hand the Wall is asked to leave.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two Forests, and nothing else on this board makes mana — the whole cost
    // is `{1}{G}` and the lands are the whole of what pays it.
    cast_from_hand(&mut engine, p0, wall_of_blossoms());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_of_blossoms()).expect("the Wall resolved");
    assert_eq!(
        pt(&engine, wall),
        (0, 4),
        "the body the card prints: power 0 and toughness 4"
    );
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "\"Defender\" reaches the battlefield object through the layers"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"When this creature enters, draw a card\": one card off the top of \
         the library, and none for anything else on the board"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Wall left the hand and the entry draw put one card back: a hand \
         of the same size is what one-for-one looks like, where a hand one \
         short would be the same library count with no draw at all"
    );
}
