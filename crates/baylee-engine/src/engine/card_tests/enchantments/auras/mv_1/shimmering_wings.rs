//! `cards/enchantments/auras/mv_1/shimmering_wings.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn shimmering_wings() -> CardIndex {
    card_index("7d892d61-5a69-4500-9fb2-7cf50728aedd")
}

/// `Shimmering Wings` ({U}, Enchantment — Aura, `Coverage::Implemented`):
/// `Enchant creature`
/// `Enchanted creature has flying.`
/// `{U}: Return this Aura to its owner's hand.`
#[test]
fn shimmering_wings_grants_flying_to_enchanted_creature_and_returns_to_hand_for_u() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), GRIZZLY_BEARS])
        .hand(0, &[shimmering_wings()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, shimmering_wings());
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

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, shimmering_wings(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, shimmering_wings()).is_none());
    assert!(in_hand(&engine, p0, shimmering_wings()).is_some());
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}
