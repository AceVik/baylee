//! Tests for Flailing Soldier.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "2c1dd6f4-2db5-449d-9ed8-579d98cfcfb8"
fn flailing_soldier() -> CardIndex {
    card_index("2c1dd6f4-2db5-449d-9ed8-579d98cfcfb8")
}

/// {1}: This creature gets +1/+1 until end of turn. Any player may activate this ability.
/// {1}: This creature gets -1/-1 until end of turn. Any player may activate this ability.
#[test]
fn flailing_soldier_can_be_pumped_or_shrunk_by_any_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[mountain()])
        .hand(0, &[flailing_soldier()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, flailing_soldier());
    pass_until(&mut engine, stack_is_empty);
    let soldier = on_battlefield(&engine, p0, flailing_soldier()).expect("Flailing Soldier");
    assert_eq!(pt(&engine, soldier), (2, 2));

    // P0 activates ability 0 (+1/+1).
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: soldier,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, soldier), (3, 3));

    // P0 passes priority so P1 can activate ability 1 (-1/-1).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: soldier,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, soldier), (2, 2));
}
