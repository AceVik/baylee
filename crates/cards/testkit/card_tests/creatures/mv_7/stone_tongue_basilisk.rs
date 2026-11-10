//! Oracle scenarios for Stone-Tongue Basilisk.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn stone_tongue_basilisk_destroys_damaged_creature_at_end_of_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("c38efbe5-7ace-41fa-b874-72e3702a833c");
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[card])
        .battlefield(1, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let basilisk = on_battlefield(&e, p0, card).unwrap();
    let wall = on_battlefield(&e, p1, index::WALL_OF_STONE).unwrap();

    fight(&mut e, &[basilisk], &[(wall, basilisk)]);

    assert!(on_battlefield(&e, p0, card).is_some());
    assert!(on_battlefield(&e, p1, index::WALL_OF_STONE).is_none());
}

#[test]
fn stone_tongue_basilisk_at_threshold_requires_every_able_creature_to_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("c38efbe5-7ace-41fa-b874-72e3702a833c");
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[card])
        .battlefield(1, &[index::WALL_OF_STONE, index::GRAY_OGRE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    seed_graveyard(&mut e, p0, 7);

    let basilisk = on_battlefield(&e, p0, card).unwrap();
    let wall = on_battlefield(&e, p1, index::WALL_OF_STONE).unwrap();
    let ogre = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(basilisk, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "declaring no blockers is illegal at threshold"
    );
    assert!(
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, basilisk)],
            },
        )
        .is_err(),
        "omitting any able blocker is illegal at threshold"
    );
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, basilisk), (ogre, basilisk)],
        },
    )
    .expect("blocking with every able creature is legal");
}
