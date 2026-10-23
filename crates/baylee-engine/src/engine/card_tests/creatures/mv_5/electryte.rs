#[allow(clippy::wildcard_imports)]
use super::*;

fn electryte() -> CardIndex {
    card_index("5ff0c25f-4593-460b-9d7f-d9c37f135a18")
}

#[test]
fn electryte_deals_damage_to_blocking_creatures_when_hitting_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[electryte(), rib_cage_spider()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elec = on_battlefield(&engine, p0, electryte()).expect("seated");
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("seated");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elec, baylee_core::ids::Defender::Player(p1)),
                    (spider, baylee_core::ids::Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elephant, spider)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().players[1].life == 17 && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p1, wild_elephant()).is_none());
}
