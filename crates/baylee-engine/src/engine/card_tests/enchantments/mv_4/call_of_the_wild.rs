//! `cards/enchantments/mv_4/call_of_the_wild.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn call_of_the_wild() -> CardIndex {
    card_index("882ee72e-2149-4778-a81c-8bf09beb5d66")
}

/// `Call of the Wild` ({2}{G}{G}, Enchantment) activates for `{2}{G}{G}` to
/// reveal the top card of your library and put it onto the battlefield if it's
/// a creature card.
#[test]
fn call_of_the_wild_puts_revealed_top_creature_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[call_of_the_wild()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, call_of_the_wild());
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, call_of_the_wild(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}
