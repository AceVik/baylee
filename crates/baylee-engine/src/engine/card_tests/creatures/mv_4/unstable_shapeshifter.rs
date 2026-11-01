//! `cards/creatures/mv_4/unstable_shapeshifter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn unstable_shapeshifter() -> CardIndex {
    card_index("6a71deb3-6659-47ea-8687-136cf75fa381")
}

/// `Unstable Shapeshifter` ({3}{U}, 0/1 Creature — Shapeshifter,
/// `Coverage::Implemented`):
/// `Whenever another creature enters, this creature becomes a copy of that
/// creature, except it has this ability.`
#[test]
fn unstable_shapeshifter_copies_each_entering_creature_and_keeps_its_trigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                unstable_shapeshifter(),
            ],
        )
        .hand(
            0,
            &[
                GRIZZLY_BEARS,
                HILL_GIANT,
                baylee_core::generated::index::SPELLBOOK,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shifter = on_battlefield(&engine, p0, unstable_shapeshifter()).unwrap();
    let ch0 = engine.state().object(shifter).unwrap().characteristics();
    assert_eq!((ch0.power, ch0.toughness), (Some(0), Some(1)));

    cast_from_hand(&mut engine, p0, baylee_core::generated::index::SPELLBOOK);
    pass_until(&mut engine, stack_is_empty);
    let unchanged = engine.state().object(shifter).unwrap().characteristics();
    assert_eq!(
        (unchanged.power, unchanged.toughness),
        (Some(0), Some(1)),
        "noncreatures do not trigger the copy"
    );

    // Cast Grizzly Bears (2/2) -> Unstable Shapeshifter becomes a 2/2 copy.
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    let ch1 = engine.state().object(shifter).unwrap().characteristics();
    assert_eq!((ch1.power, ch1.toughness), (Some(2), Some(2)));

    // Cast Hill Giant (3/3) -> Unstable Shapeshifter kept its ability and becomes a 3/3 copy.
    cast_from_hand(&mut engine, p0, HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);

    let ch2 = engine.state().object(shifter).unwrap().characteristics();
    assert_eq!((ch2.power, ch2.toughness), (Some(3), Some(3)));
}
