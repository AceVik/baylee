//! `cards/enchantments/auras/mv_2/conviction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn conviction() -> CardIndex {
    card_index("4374b6b1-e691-4823-802d-8de8530aa461")
}

/// Conviction (`4374b6b1-e691-4823-802d-8de8530aa461`) — {1}{W} Enchantment — Aura (Implemented):
/// "Enchant creature. Enchanted creature gets +1/+3. {W}: Return this Aura to its owner's hand."
#[test]
fn conviction_buffs_enchanted_creature_and_returns_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                index::GRIZZLY_BEARS,
                index::SAVANNAH_LIONS,
            ],
        )
        .hand(0, &[conviction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, conviction());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 5));
    let lions = on_battlefield(&engine, P0, index::SAVANNAH_LIONS).unwrap();
    assert_eq!(pt(&engine, lions), (2, 1));

    activate(&mut engine, P0, conviction(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, conviction()).is_some());
    assert_eq!(pt(&engine, bears), (2, 2));
}
