//! `cards/creatures/artifacts/mv_3/watcher_of_the_wayside.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn watcher_of_the_wayside() -> CardIndex {
    card_index("b30eafed-2755-4423-9592-c8b83c68804b")
}

/// Watcher of the Wayside — "When this creature enters, target player mills
/// two cards. You gain 2 life." Aimed at the opponent: their library loses
/// two to their graveyard, the controller's life goes up.
#[test]
fn watcher_of_the_wayside_mills_the_target_player_and_gains_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[watcher_of_the_wayside()], &[forest(), forest(), forest()]);
    let theirs_before = library_of(&engine, p1).len();
    let mine_before = library_of(&engine, p0).len();

    cast_from_hand(&mut engine, p0, watcher_of_the_wayside());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let offered = aim_at_player(&mut engine, p0, p1);
    assert!(offered.contains(&p0) && offered.contains(&p1));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_of(&engine, p1).len(), theirs_before - 2);
    assert_eq!(engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(), 2);
    assert_eq!(library_of(&engine, p0).len(), mine_before);
    assert_eq!(life_of(&engine, 0), 22);
    assert_eq!(life_of(&engine, 1), 20);
    let watcher = on_battlefield(&engine, p0, watcher_of_the_wayside()).expect("it resolved");
    assert_eq!(pt(&engine, watcher), (3, 2));
}

/// The target may be its own controller.
#[test]
fn watcher_of_the_wayside_may_mill_its_own_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[watcher_of_the_wayside()], &[forest(), forest(), forest()]);
    let mine_before = library_of(&engine, p0).len();

    cast_from_hand(&mut engine, p0, watcher_of_the_wayside());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at_player(&mut engine, p0, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_of(&engine, p0).len(), mine_before - 2);
    assert_eq!(engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 2);
    assert_eq!(life_of(&engine, 0), 22);
}
