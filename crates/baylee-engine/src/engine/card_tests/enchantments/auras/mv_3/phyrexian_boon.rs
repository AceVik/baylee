//! `cards/enchantments/auras/mv_3/phyrexian_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOG_IMP, HILL_GIANT};

fn phyrexian_boon() -> CardIndex {
    card_index("a18e6468-b8a2-4681-b8fa-223ab5f3061a")
}

/// Phyrexian Boon ({2}{B} Enchantment — Aura):
/// "Enchant creature"
/// "Enchanted creature gets +2/+1 as long as it's black. Otherwise, it gets -1/-2."
#[test]
fn phyrexian_boon_pumps_black_creature_and_shrinks_nonblack_creature() {
    let p0 = PlayerId::new(0);

    // 1. Enchant a black creature (Bog Imp, 1/1) -> becomes 3/2.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), BOG_IMP])
        .hand(0, &[phyrexian_boon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let imp = on_battlefield(&engine, p0, BOG_IMP).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, phyrexian_boon());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![imp],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, imp), (3, 2));

    // 2. Enchant a nonblack creature (Hill Giant, 3/3) -> becomes 2/1.
    let mut engine2 = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), HILL_GIANT])
        .hand(0, &[phyrexian_boon()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let giant = on_battlefield(&engine2, p0, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine2, p0);
    cast_with_floating(&mut engine2, p0, phyrexian_boon());
    engine2
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine2, stack_is_empty);
    assert_eq!(pt(&engine2, giant), (2, 1));
}
