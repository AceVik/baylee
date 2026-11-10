//! `cards/lands/battle/canopy_vista.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Canopy Vista is a battle land: "This land enters tapped unless you control
/// two or more basic lands", and as a Forest Plains it taps for {G} or {W}.
/// One Forest and two Forests are the whole of that filter's bound, and the
/// pair is its own control — the same card played off the same opening, with
/// the number of basics as the only difference, so a land that always entered
/// tapped fails the second branch and one that never did fails the first.
/// The untapped half then makes a mana, which is what says the permanent that
/// arrived untapped is the Forest Plains the card prints; the tapped half
/// offers nothing at all, because a land that came in tapped is not untapped
/// until its controller's next untap step (CR 502.2).
#[test]
fn canopy_vista_enters_tapped_under_one_basic_land_and_untapped_under_two() {
    let p0 = PlayerId::new(0);

    // One basic land: "unless you control two or more basic lands" is not met.
    let mut lean = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[canopy_vista()])
        .start();
    keep_mulligans(&mut lean);
    reach_main_phase(&mut lean, p0);
    let one = play_land(&mut lean, p0, canopy_vista());
    pass_until(&mut lean, |e| at_rest(e, p0));
    assert!(
        entered_tapped(&lean, one),
        "one Forest is one basic land, so the battle land enters tapped"
    );
    assert_eq!(
        tap_mana_where(&mut lean, p0, |id| id == one),
        0,
        "and a land that entered tapped has no mana to give this turn"
    );
    assert_eq!(
        lean.state().players[0].mana_pool.total(),
        0,
        "so the pool is empty, off a board that tapped nothing"
    );

    // Two basic lands: the same card, the same turn, the count one higher.
    let mut rich = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[canopy_vista()])
        .start();
    keep_mulligans(&mut rich);
    reach_main_phase(&mut rich, p0);
    let two = play_land(&mut rich, p0, canopy_vista());
    pass_until(&mut rich, |e| at_rest(e, p0));
    assert!(
        !entered_tapped(&rich, two),
        "two Forests meet the printed bound, so it enters untapped"
    );

    // The untapped half is the Forest Plains it prints: one mana, and it is
    // green or white, which is what "Add {G} or {W}" leaves to a choice.
    let taken = tap_mana_where(&mut rich, p0, |id| id == two);
    assert_eq!(taken, 1, "the land is a mana source exactly once");
    let pool = &rich.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Green) + pool.available(ManaColor::White),
        1,
        "and it is one of the two colors its basic land types print"
    );
    assert!(is_tapped(&rich, two), "the tap symbol is what paid for it");
}
