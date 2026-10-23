#[allow(clippy::wildcard_imports)]
use super::*;

fn antagonism() -> CardIndex {
    card_index("68f81fcf-2dcf-41b9-ad7c-588149feb12a")
}

#[test]
fn antagonism_deals_two_damage_to_active_player_at_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[antagonism()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, antagonism());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, antagonism()).is_some());

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[0].life, 18);
}
