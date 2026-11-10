//! `cards/lands/scorched_ruins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scorched Ruins makes four colourless off one tap, which is the whole
/// reason the card has a drawback, and the drawback is the half this file
/// cannot write: "If this land would enter, sacrifice two untapped lands
/// instead" is a replacement charging a price in permanents and
/// `EnterModifier` has no variant for it.
///
/// So the land is put onto the battlefield directly, which is the honest
/// scenario for a `Coverage::Partial` of this shape: the entry is the part
/// that does not exist, and starting the land in play neither asserts that
/// it works nor pretends it was paid for. What is asserted is the four —
/// **and** that they are four of one kind, because a land that made one of
/// each colour would have the same total.
#[test]
fn scorched_ruins_taps_for_four_colourless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scorched_ruins()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ruins = on_battlefield(&engine, p0, scorched_ruins()).expect("the land is in play");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before the tap"
    );
    activate(&mut engine, p0, scorched_ruins(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        4,
        "{{T}}: Add {{C}}{{C}}{{C}}{{C}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and all four are colourless"
    );
    assert!(is_tapped(&engine, ruins), "its own tap symbol was the cost");
}
