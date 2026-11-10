//! `cards/lands/battle/sodden_verdure.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sodden Verdure — Land — Forest Island, "This land enters tapped unless you
/// control two or more basic lands", "{T}: Add {G} or {U}".
///
/// The count is the whole card, so it is played on two boards that differ in
/// nothing but how many basic lands already stand: one Forest and it arrives
/// tapped, two Plains and it does not. The first scenario is also what says the
/// land printing two basic land *types* is no basic land itself (CR 205.4a) —
/// a filter reading types instead of the supertype would find two lands on that
/// board and turn the answer over. The untapped half then pays for the mana
/// line, with both Plains held back so that mana of a colour no Plains prints
/// can only be the Sodden Verdure's own tap.
#[test]
fn sodden_verdure_enters_tapped_on_one_basic_land_and_untapped_on_two() {
    let p0 = PlayerId::new(0);

    // One basic land is fewer than two, so the land arrives tapped.
    let mut thin = Duel::new(41, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sodden_verdure()])
        .start();
    keep_mulligans(&mut thin);
    assert!(walk_to_own_main(&mut thin, p0), "p0 reaches its own main");
    let lone = play_land(&mut thin, p0, sodden_verdure());
    assert!(
        entered_tapped(&thin, lone),
        "a single Forest is fewer than two basic lands, and the entering land \
         is not a basic land that could make up the difference"
    );

    // The same card at the same seed: two basic lands instead of one.
    let mut wide = Duel::new(41, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[sodden_verdure()])
        .start();
    keep_mulligans(&mut wide);
    assert!(walk_to_own_main(&mut wide, p0), "p0 reaches its own main");
    let open = play_land(&mut wide, p0, sodden_verdure());
    assert!(
        !entered_tapped(&wide, open),
        "\"unless you control two or more basic lands\": the two Plains are two"
    );

    // Both Plains are named as the sources to keep, so the only thing left to
    // tap is the card under test and everything in the pool came off it.
    tap_all_mana_but(&mut wide, p0, Some(plains()));
    let pool = &wide.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one land tapped for one mana");
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "a Plains nobody tapped makes no white mana, so this was no bystander's"
    );
    assert_eq!(
        pool.available(ManaColor::Green) + pool.available(ManaColor::Blue),
        1,
        "\"{{T}}: Add {{G}} or {{U}}\" — one of the two, and never a third colour"
    );
}
