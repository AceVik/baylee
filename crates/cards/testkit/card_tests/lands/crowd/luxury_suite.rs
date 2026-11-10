//! `cards/lands/crowd/luxury_suite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// All ten crowd lands, at the two table sizes their sentence is about.
///
/// A duel is the board every other land test in this file runs on, and it is
/// the one board this cycle cannot be measured from: with a single opponent
/// every one of the ten enters tapped, so a test written there would assert
/// the same branch twice and pass whatever the rule did.
#[test]
fn a_crowd_land_wants_two_opponents_and_counts_only_opponents() {
    for (i, (oracle, name)) in UNTAPPED_WITH_A_CROWD.iter().enumerate() {
        let card = card_index(oracle);
        let seed = 980 + u64::try_from(i).expect("ten rows");
        assert!(
            arrives_tapped(|| Duel::new(seed, forest()), card),
            "{name} entered untapped in a duel, where you have one opponent"
        );
        assert!(
            !arrives_tapped(|| Duel::table(seed, forest(), 3), card),
            "{name} entered tapped at a three-player table"
        );
    }

    // The half no seat count can show: two other players, one of them a
    // teammate, is **one** opponent. A rule that counted seats rather than
    // asking `is_opponent` passes every line above and fails this one.
    let suite = card_index("819e1765-8325-4e6f-89c1-63ea86de369f");
    assert!(
        arrives_tapped(
            || Duel::table(991, forest(), 3)
                .team(0, 1)
                .team(1, 1)
                .team(2, 2),
            suite
        ),
        "Luxury Suite counted a teammate as an opponent"
    );
    assert!(
        !arrives_tapped(
            || Duel::table(992, forest(), 3)
                .team(0, 1)
                .team(1, 2)
                .team(2, 3),
            suite
        ),
        "three seats on three sides is two opponents, and the land should be untapped"
    );
}
