//! `cards/creatures/artifacts/mv_1/emergency_medical_hologram.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn emergency_medical_hologram() -> CardIndex {
    card_index("8170dcbb-9879-414c-819d-ea2658ff770c")
}

/// "When this creature enters, put a +1/+1 counter on target creature. You
/// gain 1 life." Both halves resolve: the counter lands on the chosen
/// creature and the controller gains the life.
#[test]
fn emergency_medical_hologram_counters_a_creature_and_gains_a_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), grizzly_bears()])
        .hand(0, &[emergency_medical_hologram()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("bears");
    let life = engine.state().players[0].life;

    cast_from_hand(&mut engine, p0, emergency_medical_hologram());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, bears, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, bears), (3, 3));
    assert_eq!(engine.state().players[0].life, life + 1, "and a life");
    let me = on_battlefield(&engine, p0, emergency_medical_hologram()).expect("seated");
    assert_eq!(pt(&engine, me), (0, 1), "the printed body, no counter");
}
