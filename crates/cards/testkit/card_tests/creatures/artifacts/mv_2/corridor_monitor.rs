//! `cards/creatures/artifacts/mv_2/corridor_monitor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn corridor_monitor() -> CardIndex {
    card_index("07b6ee55-a77e-4ba7-9ab7-8a8375c7acc9")
}

/// Corridor Monitor: a 1/4 `{1}{U}`; "when this creature enters, untap target
/// artifact or creature you control". A tapped Gold Myr is a legal target and
/// untaps; the opponent's Bears are not on the menu.
#[test]
fn corridor_monitor_untaps_a_target_artifact_or_creature_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[corridor_monitor()],
        &[gold_myr(), island(), forest()],
        &[grizzly_bears()],
    );
    let myr = on_battlefield(&engine, p0, gold_myr()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    activate(&mut engine, p0, gold_myr(), 0);
    assert!(is_tapped(&engine, myr), "tapped for mana");

    cast_from_hand(&mut engine, p0, corridor_monitor());
    let options = pass_until_targets(&mut engine, p0);
    let monitor = on_battlefield(&engine, p0, corridor_monitor()).expect("it resolved");
    assert_eq!(pt(&engine, monitor), (1, 4));
    assert!(options.contains(&myr), "an artifact creature of ours");
    assert!(options.contains(&monitor), "it may untap itself");
    assert!(!options.contains(&bears), "not the opponent's");
    aim_at(&mut engine, p0, myr);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, myr), "untapped");
}

/// A land is neither an artifact nor a creature: the Forests are not on the
/// menu.
#[test]
fn corridor_monitor_cannot_target_a_land() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[corridor_monitor()], &[island(), forest()], &[]);
    let forest_id = on_battlefield(&engine, p0, forest()).expect("seated");
    cast_from_hand(&mut engine, p0, corridor_monitor());
    let options = pass_until_targets(&mut engine, p0);
    assert!(!options.contains(&forest_id));
}
