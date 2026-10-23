#[allow(clippy::wildcard_imports)]
use super::*;

fn skyshroud_elite() -> CardIndex {
    card_index("bd9f2c84-0589-4ffe-9bff-ea74be4f77a4")
}

#[test]
fn skyshroud_elite_gets_plus_one_plus_two_when_opponent_controls_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[skyshroud_elite()])
        .hand(1, &[badlands()])
        .battlefield(0, &[forest(), badlands()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, skyshroud_elite());
    pass_until(&mut engine, stack_is_empty);
    let elite = on_battlefield(&engine, p0, skyshroud_elite()).expect("seated");
    assert_eq!(pt(&engine, elite), (1, 1));

    // Opponent plays a nonbasic land (Badlands): Skyshroud Elite becomes 2/3.
    reach_their_main_phase(&mut engine, p1);
    let bl = in_hand(&engine, p1, badlands()).expect("in hand");
    engine
        .apply(p1, PlayerAction::PlayLand { card: bl })
        .unwrap();
    assert_eq!(pt(&engine, elite), (2, 3));
}
