//! `cards/creatures/mv_6/goretusk_firebeast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn goretusk_firebeast_etb_deals_four_damage_to_target_player() {
    let firebeast = card_index("853ce64f-50b4-49a6-9aa3-f07b11cd8b69");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[firebeast])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[1].life;
    cast_from_hand(&mut engine, p0, firebeast);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
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

    assert!(on_battlefield(&engine, p0, firebeast).is_some());
    assert_eq!(engine.state().players[1].life, life_before - 4);
}
