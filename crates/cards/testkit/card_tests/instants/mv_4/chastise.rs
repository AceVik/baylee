//! `cards/instants/mv_4/chastise.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn chastise_destroys_target_attacking_creature_and_gains_life_equal_to_its_power() {
    let chastise = card_index("b7553f3f-5de1-409c-a184-e12e40f017ab");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[chastise])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let life_before = engine.state().players[0].life;
    cast_from_hand(&mut engine, p0, chastise);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
    assert_eq!(engine.state().players[0].life, life_before + 3);
}
