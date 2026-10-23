#[allow(clippy::wildcard_imports)]
use super::*;

fn bouncing_beebles() -> CardIndex {
    card_index("4fa46972-d953-4328-9c31-96e134234133")
}

#[test]
fn bouncing_beebles_is_unblockable_when_opponent_controls_an_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[bouncing_beebles()])
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[sol_ring(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bouncing_beebles());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bb = on_battlefield(&engine, p0, bouncing_beebles()).unwrap();
    let legal = attack_and_collect_blocks(&mut engine, bb, p1);
    assert!(legal.is_empty());
}
