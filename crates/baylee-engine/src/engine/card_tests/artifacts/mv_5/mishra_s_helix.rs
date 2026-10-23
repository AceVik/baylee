#[allow(clippy::wildcard_imports)]
use super::*;

fn mishra_s_helix() -> CardIndex {
    card_index("6b1552f2-df64-485a-8509-b1916edf19cf")
}

#[test]
fn mishra_s_helix_taps_x_target_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_helix(), forest()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_land = on_battlefield(&engine, p1, island()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mishra_s_helix(), 0);
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, opp_land);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, opp_land));
}
