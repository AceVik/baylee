//! `cards/creatures/mv_4/carrier_pigeons.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn carrier_pigeons() -> CardIndex {
    card_index("2d30a634-43c6-4e78-ba10-7ef7ee17ac40")
}

/// `Carrier Pigeons` ({3}{W}, 1/1 Creature — Bird) has flying and draws a card
/// at the beginning of the next turn's upkeep when it enters.
#[test]
fn carrier_pigeons_enters_with_flying_and_draws_a_card_at_next_turns_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 4])
        .hand(0, &[carrier_pigeons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, carrier_pigeons());
    pass_until(&mut engine, stack_is_empty);
    let pigeons = on_battlefield(&engine, p0, carrier_pigeons()).expect("Carrier Pigeons enters");
    assert_eq!(pt(&engine, pigeons), (1, 1));
    assert!(keywords(&engine, pigeons).contains(KeywordSet::FLYING));

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );

    // Only the next turn's upkeep, once: on P0's own next turn it draws just the draw step's card.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "the Pigeons' card, then the ordinary draw; no second Pigeons card"
    );
}
