#[allow(clippy::wildcard_imports)]
use super::*;

fn fortitude() -> CardIndex {
    card_index("5017c07d-8ad5-4ee2-91d3-974811cdff75")
}

#[test]
fn fortitude_sacrifices_forest_to_regenerate_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[fortitude()])
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    cast_from_hand(&mut engine, p0, fortitude());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, elves);
    }
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, fortitude()).expect("attached");
    kill(&mut engine, aura);

    assert!(in_hand(&engine, p0, fortitude()).is_some());
}
