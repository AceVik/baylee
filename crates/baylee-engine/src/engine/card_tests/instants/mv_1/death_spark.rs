//! `cards/instants/mv_1/death_spark.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn death_spark() -> CardIndex {
    card_index("4b23a89c-89ba-41ad-b894-ec02221689eb")
}

fn mons_s_goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

/// `Death Spark` ({R}, Instant, `Coverage::Partial`) deals 1 damage to any
/// target, and at the beginning of your upkeep, if it is in your graveyard with
/// a creature card above it, you may pay `{1}` to return it to your hand.
#[test]
fn death_spark_deals_one_damage_and_returns_from_graveyard_on_upkeep_when_creature_is_above_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[death_spark(), mons_s_goblin_raiders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, death_spark());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    let spark = in_graveyard(&engine, p0, death_spark()).expect("Death Spark in graveyard");

    // Put a creature card above Death Spark in P0's graveyard.
    let raiders = in_hand(&engine, p0, mons_s_goblin_raiders()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            raiders,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();

    // Advance to P0's next upkeep; pay {1} to return Death Spark to hand.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(spark).unwrap().zone, Zone::Hand);
}
