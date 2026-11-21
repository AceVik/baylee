//! `cards/enchantments/mv_2/forgotten_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn forgotten_harvest() -> CardIndex {
    card_index("9e5de2ed-e8a1-4d52-832f-c4a1bfdf2ae5")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Upkeep trigger: target creature, then optionally exile a land card (nonlands
/// excluded) from your graveyard to put a +1/+1 counter on that creature;
/// declining leaves the land in the graveyard and adds no counter.
#[test]
fn forgotten_harvest_exiles_graveyard_land_to_put_counter_when_paid() {
    let p0 = PlayerId::new(0);
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), forest(), island(), savannah_lions()],
        &[],
        &[forgotten_harvest()],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let land1 = lk::obj(&e, p0, forest());
    let land2 = lk::obj(&e, p0, island());
    let nonland = lk::obj(&e, p0, savannah_lions());
    bury(&mut e, &[land1, land2, nonland]);

    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p0, forgotten_harvest(), &[], &[]);

    // Turn 2 upkeep: target Bear, exile land1 (nonland creature omitted from options).
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[bear], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 1));
    assert!(options.contains(&land1) && options.contains(&land2));
    assert!(!options.contains(&nonland));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land1],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, land1), Zone::Exile);
    assert_eq!(pt(&e, bear), (3, 3));
    assert_eq!(
        e.state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );

    // Turn 3 upkeep: decline by selecting no land -> land2 stays in graveyard, still 3/3.
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[bear], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, land2), Zone::Graveyard);
    assert_eq!(pt(&e, bear), (3, 3));
}
