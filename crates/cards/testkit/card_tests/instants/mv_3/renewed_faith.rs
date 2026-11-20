//! `cards/instants/mv_3/renewed_faith.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn renewed_faith() -> CardIndex {
    card_index("aa4ecefa-808e-4706-af16-4ea5af9743fc")
}

/// Renewed Faith gains 6 life when cast, or cycles for `{1}{W}` to optionally
/// gain 2 life before drawing (or decline life gain and still draw).
#[test]
fn renewed_faith_cast_gains_six_and_cycling_optionally_gains_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[renewed_faith(), renewed_faith(), renewed_faith()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands = all_on_battlefield(&engine, p0, plains());

    // 1. Cast normally for {2}{W} -> +6 life (26).
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    cast_with_floating(&mut engine, p0, renewed_faith());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 26);

    // 2. Cycle for {1}{W} and accept -> +2 life (28) BEFORE drawing 1 card.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_where(&mut engine, p0, |id| lands[3..5].contains(&id));
    activate(&mut engine, p0, renewed_faith(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    assert_eq!(life(&engine, p0), 28);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "life gain resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 3. Cycle for {1}{W} and decline -> life stays 28, still draws 1 card.
    tap_mana_where(&mut engine, p0, |id| lands[5..7].contains(&id));
    activate(&mut engine, p0, renewed_faith(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life(&engine, p0), 28);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
