#[allow(clippy::wildcard_imports)]
use super::*;

fn fecundity() -> CardIndex {
    card_index("ffa64ac6-fe55-48b8-b015-849982cd7ad8")
}

#[test]
fn fecundity_lets_controller_draw_when_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fecundity(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let state = engine.dev_state_mut(p0).unwrap();
    crate::sba::destroy(state, bear);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
