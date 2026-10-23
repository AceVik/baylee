#[allow(clippy::wildcard_imports)]
use super::*;

fn karn_silver_golem() -> CardIndex {
    card_index("8a7c6391-020a-4d7a-8abb-702631bbee56")
}

#[test]
fn karn_silver_golem_animates_noncreature_artifact_with_pt_equal_to_cmc() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[karn_silver_golem(), sol_ring(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, karn_silver_golem(), 1);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ring), (1, 1));
}
