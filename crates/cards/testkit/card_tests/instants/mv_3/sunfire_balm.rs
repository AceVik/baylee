//! `cards/instants/mv_3/sunfire_balm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn sunfire_balm() -> CardIndex {
    card_index("61d2f269-8674-4429-a07c-59972afd9ede")
}

/// Cast normally, Sunfire Balm prevents the next 4 damage that would be dealt
/// to any target this turn, and unused prevention expires at end of turn.
#[test]
fn sunfire_balm_cast_prevents_next_four_damage_this_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), mountain(), mountain()])
        .hand(0, &[sunfire_balm(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let whites = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| whites.contains(&id));
    cast_with_floating(&mut engine, p0, sunfire_balm());
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);

    // First Bolt (3 damage): fully prevented, leaving 1 prevention remaining.
    let reds = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| id == reds[0]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 20);

    // Second Bolt (3 damage): 1 prevented, 2 dealt -> life 18.
    tap_mana_where(&mut engine, p0, |id| id == reds[1]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 18);
}

/// Cycling Sunfire Balm optionally prevents the next 1 damage to any target
/// this turn (resolving before draw), and may be declined while still drawing.
#[test]
fn sunfire_balm_cycling_prevents_one_damage_or_declines() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                sunfire_balm(),
                sunfire_balm(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let whites = all_on_battlefield(&engine, p0, plains());
    let reds = all_on_battlefield(&engine, p0, mountain());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and accept -> prevents 1 of the next Bolt's 3 damage (20 -> 18).
    tap_mana_where(&mut engine, p0, |id| id == whites[0] || id == whites[1]);
    activate(&mut engine, p0, sunfire_balm(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, true);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "prevention trigger resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    tap_mana_where(&mut engine, p0, |id| id == reds[0]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 18, "1 of 3 damage prevented");

    // 2. Cycle and decline -> no prevention, full 3 damage from Bolt (18 -> 15).
    tap_mana_where(&mut engine, p0, |id| id == whites[2] || id == whites[3]);
    activate(&mut engine, p0, sunfire_balm(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    tap_mana_where(&mut engine, p0, |id| id == reds[1]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 15, "declining MayDo prevents 0 damage");
}
