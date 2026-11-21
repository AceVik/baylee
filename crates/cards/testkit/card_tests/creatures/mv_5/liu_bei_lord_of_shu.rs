//! `cards/creatures/mv_5/liu_bei_lord_of_shu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn liu_bei_lord_of_shu() -> CardIndex {
    card_index("f46416cc-0c3d-4d90-bdcb-910c5d4a2014")
}

fn zhang_fei_fierce_warrior() -> CardIndex {
    card_index("f3f815b8-75c6-40e0-9716-872f0f5c9d46")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

/// Liu Bei, Lord of Shu (f46416cc-0c3d-4d90-bdcb-910c5d4a2014) — `{3}{W}{W}` 2/4
/// Legendary Creature — Human Soldier:
/// - Horsemanship
/// - Liu Bei gets +2/+2 as long as you control a permanent named Guan Yu, Sainted
///   Warrior or a permanent named Zhang Fei, Fierce Warrior.
#[test]
fn liu_bei_lord_of_shu_gets_plus_two_plus_two_only_while_you_control_sworn_brother() {
    let mut lands = vec![plains(); 11];
    lands.extend([island(); 4]);
    lands.extend([swamp(); 3]);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &lands)
        .battlefield(1, &[zhang_fei_fierce_warrior()])
        .hand(
            0,
            &[
                liu_bei_lord_of_shu(),
                zhang_fei_fierce_warrior(),
                spark_double(),
                murder(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let plains_ids: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == plains()))
        })
        .collect();
    tap_mana_where(&mut engine, P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, P0, liu_bei_lord_of_shu());
    pass_until(&mut engine, stack_is_empty);

    let liu_bei = on_battlefield(&engine, P0, liu_bei_lord_of_shu()).expect("Liu Bei entered");
    assert!(keywords(&engine, liu_bei).contains(KeywordSet::HORSEMANSHIP));
    assert_eq!(
        pt(&engine, liu_bei),
        (2, 4),
        "opponent controlling Zhang Fei does not pump Liu Bei"
    );

    cast_with_floating(&mut engine, P0, zhang_fei_fierce_warrior());
    pass_until(&mut engine, stack_is_empty);
    let zf = on_battlefield(&engine, P0, zhang_fei_fierce_warrior()).unwrap();
    assert_eq!(pt(&engine, liu_bei), (4, 6));

    // Controlling two permanents named Zhang Fei still grants +2/+2 once, not +4/+4.
    tap_all_mana_but(&mut engine, P0, Some(swamp()));
    cast_with_floating(&mut engine, P0, spark_double());
    let _ = pass_until_targets(&mut engine, P0);
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![zf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, P0, spark_double()).unwrap();
    assert_eq!(
        pt(&engine, liu_bei),
        (4, 6),
        "controlling multiple matching permanents does not stack +2/+2"
    );

    // Destroy the copy; original Zhang Fei still keeps Liu Bei at 4/6.
    cast_from_hand(&mut engine, P0, murder());
    aim_at(&mut engine, P0, copy);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, liu_bei), (4, 6));
}

#[test]
fn liu_bei_loses_bonus_immediately_when_last_sworn_brother_leaves() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                liu_bei_lord_of_shu(),
                zhang_fei_fierce_warrior(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let liu_bei = on_battlefield(&engine, P0, liu_bei_lord_of_shu()).unwrap();
    let zf = on_battlefield(&engine, P0, zhang_fei_fierce_warrior()).unwrap();
    assert_eq!(pt(&engine, liu_bei), (4, 6));

    cast_from_hand(&mut engine, P0, murder());
    aim_at(&mut engine, P0, zf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, liu_bei), (2, 4));
}
