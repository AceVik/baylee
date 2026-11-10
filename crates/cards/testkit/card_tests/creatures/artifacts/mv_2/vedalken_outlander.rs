//! `cards/creatures/artifacts/mv_2/vedalken_outlander.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vedalken_outlander() -> CardIndex {
    card_index("814a30dc-14e4-45cb-9320-b043e686b8ba")
}

/// The red instant that stands in for "a red source".
fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

/// Vedalken Outlander: a 2/2 with protection from red. A red creature may not
/// block it; a green one may.
#[test]
fn vedalken_outlander_cannot_be_blocked_by_a_red_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[vedalken_outlander()],
        &[hurloon_minotaur(), grizzly_bears()],
    );
    let outlander = on_battlefield(&engine, p0, vedalken_outlander()).expect("seated");
    assert_eq!(pt(&engine, outlander), (2, 2));
    let minotaur = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, outlander, p1);
    assert!(
        options.iter().any(|o| o.blocker == bears),
        "green may block"
    );
    assert!(
        !options.iter().any(|o| o.blocker == minotaur),
        "red may not: {options:?}"
    );
}

/// A red spell cannot target it: Shock may name the Bears but not the
/// Outlander.
#[test]
fn vedalken_outlander_cannot_be_targeted_by_a_red_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[shock()],
        &[vedalken_outlander(), grizzly_bears(), mountain()],
        &[],
    );
    let outlander = on_battlefield(&engine, p0, vedalken_outlander()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, shock());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&bears), "the control is a legal target");
    assert!(
        !options.contains(&outlander),
        "protection from red: no red spell may target it: {options:?}"
    );
}
