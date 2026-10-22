//! `cards/enchantments/auras/mv_1/crown_of_flames.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn crown_of_flames() -> CardIndex {
    card_index("f14fb965-5ebc-427a-a5b8-149f54c64171")
}

/// Crown of Flames (f14fb965-5ebc-427a-a5b8-149f54c64171) — {R} Enchantment — Aura:
/// "Enchant creature. {R}: Enchanted creature gets +1/+0 until end of turn. {R}: Return this Aura to its owner's hand."
#[test]
fn crown_of_flames_pumps_enchanted_creature_and_returns_to_hand() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[index::GRIZZLY_BEARS, mountain(), mountain(), mountain()],
        )
        .hand(0, &[crown_of_flames()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, crown_of_flames());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, crown_of_flames(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 2));

    activate(&mut engine, P0, crown_of_flames(), 2);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P0, crown_of_flames()).is_some());
}
