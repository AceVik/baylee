//! `cards/creatures/artifacts/mv_2/malcator_s_watcher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn malcator_s_watcher() -> CardIndex {
    card_index("b0493924-dc6d-4655-865f-7e9b40f547ea")
}

/// Malcator's Watcher: a 1/1 with "Flying, vigilance". Attacking into a Bears
/// and a Storm Crow, only the Crow may block; it stays untapped.
#[test]
fn malcator_s_watcher_flies_and_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[malcator_s_watcher()],
        &[grizzly_bears(), storm_crow()],
    );
    let watcher = on_battlefield(&engine, p0, malcator_s_watcher()).expect("seated");
    assert_eq!(pt(&engine, watcher), (1, 1));
    assert!(keywords(&engine, watcher).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, watcher).contains(KeywordSet::VIGILANCE));
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, watcher, p1);
    assert!(options.iter().any(|o| o.blocker == crow));
    assert!(!options.iter().any(|o| o.blocker == bears));
    assert!(!is_tapped(&engine, watcher), "vigilance");
}

/// "When this creature dies, draw a card."
#[test]
fn malcator_s_watcher_draws_a_card_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[malcator_s_watcher()], &[]);
    let watcher = on_battlefield(&engine, p0, malcator_s_watcher()).expect("seated");
    let hand = hand_count(&engine, 0);
    let library = library_size(&engine, p0);
    kill(&mut engine, watcher);
    assert!(in_graveyard(&engine, p0, malcator_s_watcher()).is_some());
    assert_eq!(hand_count(&engine, 0), hand + 1);
    assert_eq!(library_size(&engine, p0), library - 1);
}
