#[allow(clippy::wildcard_imports)]
use super::*;

fn fatigue() -> CardIndex {
    card_index("0a88dcb6-a391-408f-8bfc-7b5b2cc34267")
}

#[test]
fn fatigue_casts_targeting_a_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[fatigue()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fatigue());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, fatigue()).is_some());
}
