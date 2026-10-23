#[allow(clippy::wildcard_imports)]
use super::*;

fn cinder_seer() -> CardIndex {
    card_index("64b63797-9db0-41fa-b2eb-fbf51de0e337")
}

#[test]
fn cinder_seer_deals_damage_equal_to_red_cards_in_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .battlefield(0, &[cinder_seer(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, cinder_seer(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
}
