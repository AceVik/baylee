//! `cards/enchantments/auras/mv_1/whip_silk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

/// `Whip Silk` ({G}, Enchantment — Aura):
/// `Enchant creature`
/// `Enchanted creature has reach.`
/// `{G}: Return this Aura to its owner's hand.`
#[test]
fn whip_silk_grants_reach_to_enchanted_creature_and_returns_to_hand_for_g() {
    let whip_silk = card_index("ce2c420b-e569-4715-b65b-4a097700799d");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), GRIZZLY_BEARS])
        .hand(0, &[whip_silk])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, whip_silk);
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
            .contains(KeywordSet::REACH)
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, whip_silk, 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, whip_silk).is_none());
    assert!(in_hand(&engine, p0, whip_silk).is_some());
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::REACH)
    );
}
