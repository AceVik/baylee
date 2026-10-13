//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn vapor_prevents_damage_from_its_attacker_but_not_other_sources() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears(), llanowar_elves()])
        .battlefield(1, &[index::WALL_OF_VAPOR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let wall = on_battlefield(&e, p1, index::WALL_OF_VAPOR).unwrap();
    attack_and_collect_blocks(&mut e, bear, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bear)],
        },
    )
    .unwrap();
    assert_eq!(deal_test_damage(&mut e, bear, wall, false), 0);
    assert_eq!(deal_test_damage(&mut e, bear, wall, true), 0);
    assert_eq!(deal_test_damage(&mut e, elf, wall, false), 1);
}

#[test]
fn vapor_prevents_damage_only_from_the_creature_it_blocks_not_from_another_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears(), llanowar_elves()])
        .battlefield(1, &[index::WALL_OF_VAPOR, giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let wall = on_battlefield(&e, p1, index::WALL_OF_VAPOR).unwrap();
    let spider = on_battlefield(&e, p1, giant_spider()).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p1)), (elf, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bear), (spider, elf)],
        },
    )
    .unwrap();
    assert_eq!(
        deal_test_damage(&mut e, bear, wall, true),
        0,
        "the creature the Wall is blocking deals it no damage"
    );
    assert_eq!(
        deal_test_damage(&mut e, elf, wall, true),
        1,
        "an attacker the Wall is not blocking (the Spider is) still damages it"
    );
}
