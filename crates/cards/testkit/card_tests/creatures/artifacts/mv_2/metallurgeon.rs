//! `cards/creatures/artifacts/mv_2/metallurgeon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn metallurgeon() -> CardIndex {
    card_index("09373283-a5c1-4d8e-ad25-19cd55b1e869")
}

/// Metallurgeon: a 1/2 with "{W}, {T}: Regenerate target artifact". With a
/// shield raised on a Gold Myr, destroying the Myr leaves it on the
/// battlefield, tapped.
#[test]
fn metallurgeon_regenerates_a_target_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[metallurgeon(), gold_myr(), plains()], &[]);
    let metallurgeon_id = on_battlefield(&engine, p0, metallurgeon()).expect("seated");
    assert_eq!(pt(&engine, metallurgeon_id), (1, 2));
    let myr = on_battlefield(&engine, p0, gold_myr()).expect("seated");
    let plains_id = on_battlefield(&engine, p0, plains()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == plains_id);
    activate(&mut engine, p0, metallurgeon(), 0);
    let options = aim_at(&mut engine, p0, myr);
    assert!(
        is_tapped(&engine, metallurgeon_id),
        "{{T}} is part of the price"
    );
    assert!(options.contains(&myr));
    assert!(
        options.contains(&metallurgeon_id),
        "any artifact, itself too"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(myr).unwrap().regeneration_shields,
        1,
        "a shield stands over the Myr"
    );

    kill(&mut engine, myr);
    assert_eq!(
        engine.state().object(myr).map(|o| o.zone),
        Some(Zone::Battlefield),
        "regenerated instead of dying"
    );
    assert!(is_tapped(&engine, myr), "regeneration taps it");
}

/// Without the shield the same destruction kills the Myr.
#[test]
fn metallurgeon_without_its_shield_the_myr_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[metallurgeon(), gold_myr(), plains()], &[]);
    let myr = on_battlefield(&engine, p0, gold_myr()).expect("seated");
    kill(&mut engine, myr);
    assert!(in_graveyard(&engine, p0, gold_myr()).is_some());
}

/// Only artifacts: the Bears are not on the menu.
#[test]
fn metallurgeon_cannot_target_a_nonartifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[metallurgeon(), grizzly_bears(), plains()], &[]);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, metallurgeon(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target question")
    };
    assert!(!options.contains(&bears));
}
