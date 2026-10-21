//! Woolly Spider — `{1}{G}{G}` 2/3 Creature — Spider (Ice Age)
//!
//! Reach
//! Whenever Woolly Spider blocks a creature with flying, Woolly Spider gets
//! +0/+2 until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn woolly_spider() -> CardIndex {
    card_index("811c6f52-03cb-49a9-a3c7-7f4ae40e7155")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

#[test]
fn woolly_spider_gains_two_toughness_when_blocking_flyer_and_not_when_blocking_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[woolly_spider()])
        .battlefield(1, &[air_elemental()])
        .start();
    keep_mulligans(&mut engine);

    let spider = on_battlefield(&engine, p0, woolly_spider()).unwrap();
    let flyer = on_battlefield(&engine, p1, air_elemental()).unwrap();

    // Branch 1: blocks 4/4 Air Elemental with flying -> triggers +0/+2 (becomes 2/5) and survives 4 damage!
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(flyer, Defender::Player(p0))],
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
                blockers: vec![(spider, flyer)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, spider),
        (2, 5),
        "Woolly Spider gets +0/+2 until EOT when blocking a creature with flying"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(spider).unwrap().zone,
        Zone::Battlefield,
        "2/5 Woolly Spider survived 4 combat damage from Air Elemental"
    );

    // Branch 2: blocks ground Grizzly Bears -> does not trigger +0/+2, stays 2/3.
    let mut engine2 = Duel::new(SEED, forest())
        .battlefield(0, &[woolly_spider()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine2);
    let spider2 = on_battlefield(&engine2, p0, woolly_spider()).unwrap();
    let bears = on_battlefield(&engine2, p1, grizzly_bears()).unwrap();
    pass_until(
        &mut engine2,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine2
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine2
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider2, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine2, stack_is_empty);
    assert_eq!(
        pt(&engine2, spider2),
        (2, 3),
        "Woolly Spider does not get +0/+2 when blocking a creature without flying"
    );
}
