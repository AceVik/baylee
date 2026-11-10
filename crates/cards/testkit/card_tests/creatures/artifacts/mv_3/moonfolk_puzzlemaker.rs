//! `cards/creatures/artifacts/mv_3/moonfolk_puzzlemaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn moonfolk_puzzlemaker() -> CardIndex {
    card_index("8ee2371c-e2c3-412e-b9e9-1ec511c9d537")
}

/// Moonfolk Puzzlemaker — 1/4 flier; "Whenever this creature becomes tapped,
/// scry 1." Attacking taps it, and the scry follows.
#[test]
fn moonfolk_puzzlemaker_scries_one_when_it_becomes_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[moonfolk_puzzlemaker()]);
    let maker = on_battlefield(&engine, p0, moonfolk_puzzlemaker()).expect("seated");
    assert_eq!(pt(&engine, maker), (1, 4));
    assert!(keywords(&engine, maker).contains(KeywordSet::FLYING));
    assert!(!is_tapped(&engine, maker));
    assert!(
        !pass_has_arrange(&engine),
        "nothing is being scried before it taps"
    );

    let offered = attackers_offered(&mut engine);
    assert!(offered.contains(&maker));
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(maker, Defender::Player(PlayerId::new(1)))],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, maker), "attacking tapped it");

    let before = answer_scry(&mut engine, 1, true);
    assert_scried(&engine, &before, 1, true);
}

fn pass_has_arrange(engine: &Engine<RegistryLookup>) -> bool {
    matches!(engine.pending(), Pending::Arrange { .. })
}
