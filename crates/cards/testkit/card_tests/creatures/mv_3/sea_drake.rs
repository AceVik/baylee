//! `cards/creatures/mv_3/sea_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn sea_drake() -> CardIndex {
    card_index("44d1029f-604e-44d6-89f6-bd18344f5d79")
}

/// Sea Drake ({2}{U} 4/3 Drake):
/// "Flying; When this creature enters, return two target lands you control to
/// their owner's hand."
#[test]
fn sea_drake_returns_two_target_lands_you_control_and_flies_over_ground_blockers() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), forest()],
        &[plains(), index::GRIZZLY_BEARS],
        &[sea_drake()],
        &[],
    );
    let my_islands: Vec<ObjectId> = e
        .state()
        .battlefield_seen()
        .filter(|&id| {
            let o = e.state().object(id).unwrap();
            o.controller == lk::P0 && o.card.is_some_and(|c| c.index == island())
        })
        .collect();
    let my_forest = lk::obj(&e, lk::P0, forest());
    let their_plains = lk::obj(&e, lk::P1, plains());
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    cast_from_hand(&mut e, lk::P0, sea_drake());
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((player, min, max), (lk::P0, 2, 2));
    assert!(options.contains(&my_forest));
    assert!(!options.contains(&their_plains));

    lk::aim(&mut e, &[my_islands[0], my_forest], &[]);
    lk::settle(&mut e);

    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 2);
    assert!(in_hand(&e, lk::P0, forest()).is_some());
    assert!(in_hand(&e, lk::P0, island()).is_some());
    assert!(on_battlefield(&e, lk::P0, forest()).is_none());

    let drake = lk::obj(&e, lk::P0, sea_drake());
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[drake]);
    assert!(!lk::may_block(&offer, their_bear, drake));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 16);
}

#[test]
fn sea_drake_trigger_is_removed_from_stack_when_you_control_only_one_land() {
    let mut e = lk::game(
        SEED,
        &[island()],
        &[plains(), plains()],
        &[sea_drake()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut e, lk::P0, sea_drake());
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P0, sea_drake()).is_some());
    assert!(on_battlefield(&e, lk::P0, island()).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 0);
}
