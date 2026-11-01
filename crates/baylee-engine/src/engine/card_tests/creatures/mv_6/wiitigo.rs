//! Wiitigo — `{3}{G}{G}{G}` 0/0 Creature — Yeti (Ice Age)
//!
//! Wiitigo enters with six +1/+1 counters on it.
//! At the beginning of your upkeep, put a +1/+1 counter on Wiitigo if it has
//! blocked or been blocked since your last upkeep. Otherwise, remove a +1/+1
//! counter from it.
//! (`Coverage::Partial`: upkeep trigger unconditionally removes a +1/+1
//! counter.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn wiitigo() -> CardIndex {
    card_index("7f956a21-854e-41cc-96f9-ad774e69fa8e")
}

#[test]
fn wiitigo_enters_with_six_plus_one_plus_one_counters_and_removes_one_on_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wiitigo()])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, wiitigo()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let yeti = on_battlefield(&engine, p0, wiitigo()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(yeti)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        6,
        "Wiitigo enters with six +1/+1 counters"
    );
    assert_eq!(pt(&engine, yeti), (6, 6));

    // Advance to p0's next upkeep and resolve the upkeep trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(yeti)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5,
        "upkeep trigger removes a +1/+1 counter from Wiitigo"
    );
    assert_eq!(pt(&engine, yeti), (5, 5));
}
