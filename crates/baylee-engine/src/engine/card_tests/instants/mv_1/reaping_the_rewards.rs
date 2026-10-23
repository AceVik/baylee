use super::*;

fn reaping_the_rewards() -> CardIndex {
    card_index("b4177445-46d8-4427-a345-d786c820c3af")
}

#[test]
fn reaping_the_rewards_gains_2_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .hand(0, &[reaping_the_rewards()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, reaping_the_rewards());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert!(in_graveyard(&engine, p0, reaping_the_rewards()).is_some());
}
