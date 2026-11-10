//! `cards/enchantments/auras/mv_2/blessing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blessing: "Enchant creature" / "{W}: Enchanted creature gets +1/+1 until
/// end of turn." Attaching grants no static bonus by itself; each activation
/// of the printed ability adds +1/+1 until end of turn.
#[test]
fn blessing_pumps_the_enchanted_creature_when_activated() {
    let p0 = PlayerId::new(0);
    let blessing = blessing();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), quiet_creature()])
        .hand(0, &[blessing])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("creature seated");

    let plains_ids = all_on_battlefield(&engine, p0, plains());
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: plains_ids[0],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: plains_ids[1],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p0, blessing);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, creature),
        (1, 1),
        "Blessing grants no static bonus by itself"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: plains_ids[2],
            },
        )
        .unwrap();
    activate(&mut engine, p0, blessing, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, creature), (2, 2), "+1/+1 until end of turn");
}

/// Blessing: "until end of turn". Two activations stack, last through the
/// turn, and are gone once the next turn begins.
#[test]
fn blessing_pump_ends_with_the_turn() {
    a_pump_aura_ends_with_the_turn(blessing(), plains(), 1, (1, 1), (2, 2), (3, 3));
}

/// Blessing: "Enchant creature". Offered every creature and nothing else.
#[test]
fn blessing_enchants_only_creatures() {
    an_aura_enchants_only_creatures(blessing(), plains());
}
