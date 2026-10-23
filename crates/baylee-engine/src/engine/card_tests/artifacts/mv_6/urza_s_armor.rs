#[allow(clippy::wildcard_imports)]
use super::*;

fn urza_s_armor() -> CardIndex {
    card_index("e5fa232a-c8f2-4532-88bf-60d1223181b0")
}

#[test]
fn urza_s_armor_reduces_damage_dealt_to_you() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[urza_s_armor(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
}
