//! `cards/enchantments/mv_4/reclamation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn reclamation() -> CardIndex {
    card_index("ff942899-a70d-4fca-b125-9712aa635fff")
}

fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

fn objs(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Black creatures can't attack unless their controller sacrifices a land of
/// their choice for each attacking black creature (nonblack attacks free; both
/// controller and opponent's black creatures pay).
#[test]
fn reclamation_requires_land_sacrifice_per_attacking_black_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[reclamation(), scathe_zombies(), grizzly_bears(), forest()],
        &[scathe_zombies(), scathe_zombies(), swamp()],
        &[],
        &[],
    );
    let my_zombie = lk::obj(&e, p0, scathe_zombies());
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let my_forest = lk::obj(&e, p0, forest());

    // With 1 land, attacking with Bear (nonblack) + Zombie asks for 1 land sacrifice.
    lk::attackers_offered(&mut e);
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (my_zombie, Defender::Player(p1)),
                (my_bear, Defender::Player(p1)),
            ],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::ChooseTargets { .. }
        )
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![my_forest],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, my_forest), Zone::Graveyard);
    assert_eq!(lk::life(&e, p1), 16);

    // P1 has two black Zombies and only 1 Swamp -> cannot attack with both, can attack with 1.
    let opp_zombies = objs(&e, p1, scathe_zombies());
    let opp_swamp = lk::obj(&e, p1, swamp());
    reach_their_main_phase(&mut e, p1);
    lk::attackers_offered(&mut e);
    assert!(
        e.apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (opp_zombies[0], Defender::Player(p0)),
                    (opp_zombies[1], Defender::Player(p0)),
                ],
            },
        )
        .is_err()
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(opp_zombies[0], Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::ChooseTargets { .. }
        )
    });
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![opp_swamp],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, opp_swamp), Zone::Graveyard);
    assert_eq!(lk::life(&e, p0), 18);

    // With 0 lands remaining on P0's next combat, black Zombie cannot attack while Bear still can.
    reach_their_main_phase(&mut e, p0);
    let offered = lk::attackers_offered(&mut e);
    assert!(offered.contains(&my_bear));
    assert!(!offered.contains(&my_zombie));
}
