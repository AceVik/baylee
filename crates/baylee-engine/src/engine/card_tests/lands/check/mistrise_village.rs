//! `cards/lands/check/mistrise_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mistrise Village enters tapped unless you control a Mountain or a Forest,
/// and prints `{T}: Add {U}`; the other half of its text — making the next
/// spell uncounterable — is the `Coverage::Partial` gap and nothing here
/// presses it. The entry clause is read on both sides of its own sentence:
/// over an Island alone the Village arrives tapped, and beside a single
/// Forest it arrives standing, which one game could not say on its own — a
/// printing that had simply lost the "unless" would look identical on the
/// board that satisfies it, and one that always entered tapped would look
/// identical on the board that does not. The standing Village is then tapped
/// for mana, and the blue is the reading that matters: the Forest beside it
/// is green, so there is no other source of {U} on the table.
#[test]
fn mistrise_village_enters_tapped_without_a_forest_and_taps_for_blue_beside_one() {
    let p0 = PlayerId::new(0);

    // No Mountain and no Forest: the printed replacement does fire. Played
    // rather than seeded, because `SeatSpec::starting_battlefield` places a
    // permanent instead of letting it enter, and a placement is exactly the
    // thing no entry modifier ever looks at.
    let mut bare = Duel::new(7, island())
        .battlefield(0, &[island()])
        .hand(0, &[mistrise_village()])
        .start();
    keep_mulligans(&mut bare);
    reach_main_phase(&mut bare, p0);
    let tapped = play_land(&mut bare, p0, mistrise_village());
    assert!(
        entered_tapped(&bare, tapped),
        "an Island is neither a Mountain nor a Forest, so the Village enters tapped"
    );
    // And a tapped land is a land that makes nothing: the board floats the
    // Island's {U} alone.
    tap_all_mana(&mut bare, p0);
    assert_eq!(
        bare.state().players[0].mana_pool.total(),
        1,
        "the tapped Village contributes no mana"
    );

    // The other arm of the same sentence: one Forest is enough.
    let mut engine = Duel::new(7, island())
        .battlefield(0, &[forest()])
        .hand(0, &[mistrise_village()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let village = play_land(&mut engine, p0, mistrise_village());
    assert!(
        !entered_tapped(&engine, village),
        "a Forest beside it is exactly what the \"unless\" asks for"
    );

    // Mana first: the offer is read off the pool, not off the board.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "{{T}}: Add {{U}} — and the Forest beside it makes green, so the blue \
         has no other source on this board"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the Forest, for contrast"
    );
    assert_eq!(pool.total(), 2, "two untapped lands, two mana");
    assert!(
        is_tapped(&engine, village),
        "paying which tapped the Village"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
}
