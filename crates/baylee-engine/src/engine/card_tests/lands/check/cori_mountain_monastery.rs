//! `cards/lands/check/cori_mountain_monastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cori Mountain Monastery is a checkland with the unusual pair — "unless you
/// control a Plains **or** an Island" on a land that taps for {R} — and a
/// condition is untested until both of its branches fire, so it is played
/// twice off one board difference. The Island is the half worth having,
/// because a reader that took only the first of the two printed types would
/// pass the Plains test and fail nothing.
///
/// The file is `Coverage::Partial` for the {3}{R} impulse ability, which is
/// invisible from here by construction: nothing in this scenario reaches an
/// activated ability other than the mana one, and the entry condition is a
/// replacement that runs before any of it.
#[test]
fn cori_mountain_monastery_checks_for_a_plains_or_an_island_and_taps_for_red() {
    let p0 = PlayerId::new(0);

    // Neither type on the board: the printed default applies.
    let mut bare = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[cori_mountain_monastery()])
        .start();
    keep_mulligans(&mut bare);
    assert!(walk_to_own_main(&mut bare, p0), "p0 reaches its own main");
    let tapped = play_land(&mut bare, p0, cori_mountain_monastery());
    assert!(
        entered_tapped(&bare, tapped),
        "a Forest is neither a Plains nor an Island"
    );

    // An Island, which is the second of the two types the card names.
    let mut checked = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[cori_mountain_monastery()])
        .start();
    keep_mulligans(&mut checked);
    assert!(
        walk_to_own_main(&mut checked, p0),
        "p0 reaches its own main"
    );
    let untapped = play_land(&mut checked, p0, cori_mountain_monastery());
    assert!(
        !entered_tapped(&checked, untapped),
        "\"unless you control a Plains or an Island\" — the Island is the or"
    );

    activate(&mut checked, p0, cori_mountain_monastery(), 0);
    assert_eq!(
        checked.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "{{T}}: Add {{R}}"
    );
    assert_eq!(
        checked.state().players[0].mana_pool.total(),
        1,
        "and nothing else: this land makes one colour"
    );
}
