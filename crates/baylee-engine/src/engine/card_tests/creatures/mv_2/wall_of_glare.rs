//! `cards/creatures/mv_2/wall_of_glare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wall_of_glare() -> CardIndex {
    card_index("a59897cb-7ead-494a-a1d0-c9baf79bf18b")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn wall_of_glare_has_defender_and_can_block_any_number_of_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[wall_of_glare()])
        .battlefield(1, &[grizzly_bears(), gray_ogre(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Wall of Glare for {1}{W}.
    cast_from_hand(&mut engine, p0, wall_of_glare());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_glare()).expect("Wall of Glare resolved");
    assert_eq!(pt(&engine, wall), (0, 5));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    // Advance to p1's combat: p1 attacks with all three creatures (2/2 + 2/2 + 3/3).
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bear, Defender::Player(p0)),
                    (ogre, Defender::Player(p0)),
                    (giant, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { capacity, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        capacity,
        vec![crate::choice::BlockCapacity {
            blocker: wall,
            most: None,
        }],
        "Wall of Glare has unlimited block capacity"
    );

    // Wall of Glare blocks all three attackers at once.
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bear), (wall, ogre), (wall, giant)],
            },
        )
        .expect("Wall of Glare can block all three attackers simultaneously");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "all three attackers were blocked by Wall of Glare, so p0 took 0 combat damage"
    );
    assert!(
        in_graveyard(&engine, p0, wall_of_glare()).is_some(),
        "Wall of Glare took 2 + 2 + 3 = 7 damage against 5 toughness and died"
    );
}
