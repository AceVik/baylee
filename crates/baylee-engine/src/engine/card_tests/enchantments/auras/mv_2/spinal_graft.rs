//! `cards/enchantments/auras/mv_2/spinal_graft.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GIANT_GROWTH, GRIZZLY_BEARS};

fn spinal_graft() -> CardIndex {
    card_index("70b5eca0-c61b-4066-ae5f-90948912c04e")
}

/// `Spinal Graft` ({1}{B}, Enchantment — Aura, `Coverage::Partial`):
/// `Enchant creature`
/// `Enchanted creature gets +3/+3.`
/// `When enchanted creature becomes the target of a spell or ability, destroy
/// that creature. It can't be regenerated.`
#[test]
fn spinal_graft_gives_plus_three_plus_three_and_destroys_enchanted_creature_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), forest(), GRIZZLY_BEARS])
        .hand(0, &[spinal_graft(), GIANT_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, spinal_graft());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ch = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(5), Some(5)));

    cast_from_hand(&mut engine, p0, GIANT_GROWTH);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}
