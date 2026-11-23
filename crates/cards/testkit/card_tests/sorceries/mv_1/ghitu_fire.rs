//! `cards/sorceries/mv_1/ghitu_fire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c15ae9d8-b423-4628-ad88-3f44515e0c77"
fn ghitu_fire() -> CardIndex {
    card_index("c15ae9d8-b423-4628-ad88-3f44515e0c77")
}

/// The optional {2} buys instant timing, not a different effect or a different
/// X. Five Mountains on the opponent's turn therefore pay {4}{R}: X = 2 plus
/// the optional cost, with the red mana covering the printed {R}.
#[test]
fn ghitu_fire_with_its_optional_flash_cost_deals_x_on_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[ghitu_fire()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 gives p0 priority");
    tap_all_mana(&mut engine, p0);

    cast_sorcery_with_optional_flash(&mut engine, p0, ghitu_fire());
    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("Ghitu Fire asks for X, got {:?}", engine.pending());
    };
    assert!(max >= 2, "five mana pays X = 2 plus {{2}} and {{R}}");
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("X = 2 is affordable through the optional route");
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("Ghitu Fire asks for a target, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&p1), "the opponent is a target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18, "X = 2 deals two");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all five mana paid the printed cost and optional flash cost"
    );
}
