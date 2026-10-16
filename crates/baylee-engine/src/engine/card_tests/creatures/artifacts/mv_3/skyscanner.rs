//! `cards/creatures/artifacts/mv_3/skyscanner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn skyscanner() -> CardIndex {
    card_index("974f788a-039f-4310-a2fe-16b14a1e2d35")
}

/// Skyscanner — 1/1 flier; "When this creature enters, draw a card".
#[test]
fn skyscanner_draws_a_card_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[skyscanner()], &[forest(), forest(), forest()]);
    let library_before = library_of(&engine, p0).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_before, 1);

    cast_and_settle(&mut engine, skyscanner());

    let scanner = on_battlefield(&engine, p0, skyscanner()).expect("it resolved");
    assert_eq!(pt(&engine, scanner), (1, 1));
    assert!(keywords(&engine, scanner).contains(KeywordSet::FLYING));
    assert_eq!(
        library_of(&engine, p0).len(),
        library_before - 1,
        "one card left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "the spell left the hand and the drawn card replaced it"
    );
}

/// Skyscanner cannot be blocked by a ground creature.
#[test]
fn skyscanner_cannot_be_blocked_by_a_ground_creature() {
    let (engine, [scanner, ground], options) = evasion_probe(skyscanner(), &[quiet_creature()]);
    let blocker = on_battlefield(&engine, PlayerId::new(1), quiet_creature()).expect("seated");
    assert!(may_block(&options, blocker, ground));
    assert!(!may_block(&options, blocker, scanner));
}
