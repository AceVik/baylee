#[allow(clippy::wildcard_imports)]
use super::*;

fn corrupt() -> CardIndex {
    card_index("a88a1c53-7de3-423a-a95a-24ed5666ab96")
}

#[test]
fn corrupt_drains_for_number_of_swamps_controlled() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[corrupt()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_at_player(&mut engine, corrupt(), p1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 14);
    assert_eq!(engine.state().players[0].life, 26);
}
