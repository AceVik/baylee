#[allow(clippy::wildcard_imports)]
use super::*;

fn rivalry() -> CardIndex {
    card_index("da77cf1b-eaeb-4a60-aa16-a1217ce8b550")
}

#[test]
fn rivalry_deals_damage_on_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[rivalry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rivalry());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 18);
}
