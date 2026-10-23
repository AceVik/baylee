//! `cards/instants/mv_3/forbid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn forbid() -> CardIndex {
    card_index("6e26a525-2584-490d-b106-621c1a30e1fd")
}

#[test]
fn forbid_counters_target_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[mountain()])
        .hand(0, &[forbid()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let stack_bolt = on_stack(&engine, lightning_bolt()).unwrap();
    cast_from_hand(&mut engine, p0, forbid());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[stack_bolt], &[])
    });

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, p0, forbid()).is_some());
}
