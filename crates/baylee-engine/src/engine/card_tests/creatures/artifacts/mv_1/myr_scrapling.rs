//! `cards/creatures/artifacts/mv_1/myr_scrapling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn myr_scrapling() -> CardIndex {
    card_index("1a4ffc1e-2f5e-446a-b1e8-32385b3c083b")
}

/// "Sacrifice this creature: Put a +1/+1 counter on target creature." Free
/// to activate; the counter lands on the chosen creature and the Scrapling
/// is in the graveyard.
#[test]
fn myr_scrapling_sacrifices_for_a_counter_on_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[myr_scrapling(), grizzly_bears()], &[gray_ogre()]);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("bears");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");

    activate(&mut engine, p0, myr_scrapling(), 0);
    let options = aim_at(&mut engine, p0, bears);
    assert!(
        options.contains(&bears) && options.contains(&ogre),
        "any creature"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, bears, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, bears), (3, 3));
    assert_eq!(counters_on(&engine, ogre, CounterKind::P1P1), 0);
    assert!(in_graveyard(&engine, p0, myr_scrapling()).is_some());
}
