//! `cards/creatures/artifacts/mv_2/hovermyr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hovermyr() -> CardIndex {
    card_index("b58722c5-84f2-43c2-be7e-3577c2f2a60a")
}

/// Hovermyr: a 1/2 with "Flying, vigilance". Attacking into a Bears and a
/// Storm Crow, only the flier may block it; it does not tap to attack; and
/// unblocked it deals one.
#[test]
fn hovermyr_flies_and_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[hovermyr()], &[grizzly_bears(), storm_crow()]);
    let myr = on_battlefield(&engine, p0, hovermyr()).expect("seated");
    assert_eq!(pt(&engine, myr), (1, 2));
    assert!(keywords(&engine, myr).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, myr).contains(KeywordSet::VIGILANCE));
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");

    let options = attack_and_collect_blocks(&mut engine, myr, p1);
    assert!(
        options.iter().any(|o| o.blocker == crow),
        "a flier may block a flier"
    );
    assert!(
        !options.iter().any(|o| o.blocker == bears),
        "a ground creature may not"
    );
    assert!(
        !is_tapped(&engine, myr),
        "vigilance: attacking does not tap"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 19);
    assert!(!is_tapped(&engine, myr));
}
