//! `cards/creatures/mv_3/wall_of_vipers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wall_of_vipers() -> CardIndex {
    card_index("4fcfe370-ba33-438f-bd96-2ae560f59df9")
}

/// "{3}: Destroy this creature and target creature it's blocking. Any
/// player may activate this ability." The Wall blocks the Bears and lets
/// the Ogre through, and the attacker, not the Wall's controller, pays {3}:
/// the only creature it may aim at is the one the Wall is blocking (not
/// the unblocked Ogre, not the attacker's Elves at home, not the Wall), and
/// the Wall goes with it.
#[test]
fn any_player_destroys_wall_of_vipers_and_the_creature_it_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                grizzly_bears(),
                gray_ogre(),
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[wall_of_vipers()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bears = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let elves = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let wall = on_battlefield(&e, p1, wall_of_vipers()).unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(p1)), (ogre, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bears)],
        },
    )
    .unwrap();

    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority after the blocks, got {:?}", e.pending())
    };
    assert_eq!(player, p0, "the active player is the first to act");
    assert_eq!(tap_mana_where(&mut e, p0, |id| id != elves), 3);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("priority after the taps, got {:?}", e.pending())
    };
    assert!(
        legal.abilities.contains(&(wall, 0)),
        "the opponent may activate the Wall's ability"
    );
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: wall,
            ability_index: 0,
        },
    )
    .expect("the attacker pays {3}");
    assert_eq!(aim_at(&mut e, p0, bears), vec![bears]);
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, p0, grizzly_bears()).is_some(),
        "the blocked Bears"
    );
    assert!(
        in_graveyard(&e, p1, wall_of_vipers()).is_some(),
        "the Wall itself"
    );
    assert!(on_battlefield(&e, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&e, p0, llanowar_elves()).is_some());
}
