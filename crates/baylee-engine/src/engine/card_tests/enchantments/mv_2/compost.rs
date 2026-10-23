#[allow(clippy::wildcard_imports)]
use super::*;

fn compost() -> CardIndex {
    card_index("4681bcc2-8a46-44cd-8b04-6ab140904dea")
}

#[test]
fn compost_draws_card_when_opponents_black_creature_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[compost()])
        .battlefield(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let goblin = on_battlefield(&engine, p1, festering_goblin()).unwrap();

    let state = engine.dev_state_mut(p0).unwrap();
    crate::sba::destroy(state, goblin);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p1, goblin);
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
