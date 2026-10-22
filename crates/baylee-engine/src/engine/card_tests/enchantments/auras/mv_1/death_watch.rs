//! `cards/enchantments/auras/mv_1/death_watch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn death_watch() -> CardIndex {
    card_index("6f5b6cba-3b57-4e6a-afa7-0cb62e5cc7d4")
}

#[test]
fn death_watch_drains_controller_and_gains_life_when_enchanted_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[death_watch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, death_watch());
    aim_at(&mut engine, p0, opp_bear);
    pass_until(&mut engine, stack_is_empty);

    kill(&mut engine, opp_bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);
}
