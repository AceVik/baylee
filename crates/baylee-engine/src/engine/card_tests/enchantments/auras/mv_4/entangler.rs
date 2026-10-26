//! `cards/enchantments/auras/mv_4/entangler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn entangler() -> CardIndex {
    card_index("285cf707-df0c-4eb9-8f60-5d052ee8eb8c")
}

#[test]
fn entangler_lets_enchanted_creature_block_any_number_of_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[entangler()])
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), index::WALL_OF_STONE],
        )
        .battlefield(
            1,
            &[index::GRIZZLY_BEARS, index::GRAY_OGRE, llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, index::WALL_OF_STONE).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, entangler());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears, Defender::Player(p0)),
                    (ogre, Defender::Player(p0)),
                    (elves, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bears), (wall, ogre), (wall, elves)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, index::WALL_OF_STONE).is_some());
    assert_eq!(engine.state().players[0].life, 20);
}
