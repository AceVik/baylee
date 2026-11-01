//! Misguided Rage — `{2}{R}` Sorcery (SCG).
//! Oracle: "Target player sacrifices a permanent of their choice."

use super::*;

fn misguided_rage() -> CardIndex {
    card_index("f5c77355-96dc-46af-a7de-f547a1368ff1")
}

#[test]
fn misguided_rage_forces_target_player_to_sacrifice_a_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let rage = misguided_rage();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre(), forest()])
        .hand(0, &[rage])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, rage);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p1, forest()).is_some());
}
