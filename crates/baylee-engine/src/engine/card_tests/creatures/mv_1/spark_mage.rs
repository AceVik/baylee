//! Oracle scenarios for Spark Mage.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn spark_mage_deals_1_damage_to_target_creature_damaged_player_controls() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("f427a1ed-d7ca-40e5-a759-a1096bad166d");
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[card])
        .battlefield(1, &[index::RAGING_GOBLIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let mage = on_battlefield(&e, p0, card).unwrap();
    let goblin = on_battlefield(&e, p1, index::RAGING_GOBLIN).unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(mage, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![goblin],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p1, index::RAGING_GOBLIN).is_none());
}
