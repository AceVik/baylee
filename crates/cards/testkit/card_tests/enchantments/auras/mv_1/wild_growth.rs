//! `cards/enchantments/auras/mv_1/wild_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wild Growth: "Enchant land" / "Whenever enchanted land is tapped for
/// mana, its controller adds an additional {G}." Tapping the enchanted
/// Plains for mana makes its own {W} plus one extra {G}.
#[test]
fn wild_growth_adds_an_additional_green_when_the_enchanted_land_taps_for_mana() {
    let p0 = PlayerId::new(0);
    let wild_growth = wild_growth();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[wild_growth])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let plains_id = on_battlefield(&engine, p0, plains()).expect("the Plains");
    let forest_id = on_battlefield(&engine, p0, forest()).expect("the Forest");

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forest_id })
        .unwrap();
    cast_with_floating(&mut engine, p0, wild_growth);
    aim_at(&mut engine, p0, plains_id);
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plains_id })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "the Plains' own mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "Wild Growth's additional {{G}}"
    );
}
