//! Oracle scenarios for Powerstone Minefield.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn powerstone_minefield() -> CardIndex {
    card_index("3a8a67ad-e4ff-4b55-9976-a2f13cc9b41b")
}

#[test]
fn powerstone_minefield_deals_2_damage_to_attacking_and_blocking_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::PLAINS,
                index::PLAINS,
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[powerstone_minefield()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    let ogre = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();

    cast_from_hand(&mut e, p0, powerstone_minefield());
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(p1)), (giant, Defender::Player(p1))],
        },
    )
    .unwrap();
    // Resolve Powerstone Minefield attack triggers before blockers.
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some(),
        "2/2 Grizzly Bears died to 2 damage from Powerstone Minefield on attack"
    );
    assert_eq!(
        e.state().object(giant).unwrap().damage,
        2,
        "3/3 Hill Giant has 2 damage marked on it"
    );

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(ogre, giant)],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, p1, index::GRAY_OGRE).is_some(),
        "2/2 Gray Ogre died to 2 damage from Powerstone Minefield on block"
    );
}
