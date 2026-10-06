//! `cards/enchantments/auras/mv_1/holy_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Holy Armor: "Enchant creature" / "Enchanted creature gets +0/+2." /
/// "{W}: Enchanted creature gets +0/+1 until end of turn." The static
/// bonus applies the instant it attaches; the activated ability stacks on
/// top of it.
#[test]
fn holy_armor_grants_a_static_boost_and_can_be_pumped_further() {
    let p0 = PlayerId::new(0);
    let holy_armor = holy_armor();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), quiet_creature()])
        .hand(0, &[holy_armor])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("creature seated");
    assert_eq!(pt(&engine, creature), (1, 1));

    let plains_ids = all_on_battlefield(&engine, p0, plains());
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: plains_ids[0],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p0, holy_armor);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, creature), (1, 3), "static +0/+2");

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: plains_ids[1],
            },
        )
        .unwrap();
    activate(&mut engine, p0, holy_armor, 2);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, creature),
        (1, 4),
        "plus +0/+1 until end of turn"
    );
}
