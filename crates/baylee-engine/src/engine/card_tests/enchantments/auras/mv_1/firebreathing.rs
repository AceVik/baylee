//! `cards/enchantments/auras/mv_1/firebreathing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Firebreathing: "Enchant creature" / "{R}: Enchanted creature gets +1/+0
/// until end of turn." Attaching alone changes nothing; each activation adds
/// +1/+0 until end of turn.
#[test]
fn firebreathing_pumps_power_when_activated() {
    let p0 = PlayerId::new(0);
    let firebreathing = firebreathing();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), quiet_creature()])
        .hand(0, &[firebreathing])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("creature seated");

    let mountain_ids = all_on_battlefield(&engine, p0, mountain());
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: mountain_ids[0],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p0, firebreathing);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, creature),
        (1, 1),
        "attaching alone changes nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: mountain_ids[1],
            },
        )
        .unwrap();
    activate(&mut engine, p0, firebreathing, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, creature), (2, 1), "+1/+0 until end of turn");
}
