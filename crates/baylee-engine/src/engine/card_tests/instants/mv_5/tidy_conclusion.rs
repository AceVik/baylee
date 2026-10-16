//! `cards/instants/mv_5/tidy_conclusion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d73adc2e-c458-40a9-b10b-7d2133778717"
fn tidy_conclusion() -> CardIndex {
    card_index("d73adc2e-c458-40a9-b10b-7d2133778717")
}

/// Tidy Conclusion: "Destroy target creature. You gain 1 life for each
/// artifact you control."
///
/// Two artifacts of the caster's gain two; the opponent's artifact does not
/// count.
#[test]
fn tidy_conclusion_gains_life_for_each_artifact_you_control() {
    let mut engine = table(
        935,
        &[tidy_conclusion()],
        &[swamp(); 5],
        &[quiet_artifact(), baleful_strix()],
        &[gray_ogre(), quiet_artifact()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, tidy_conclusion(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some(), "destroyed");
    assert_eq!(
        life_of(&engine, me()),
        22,
        "two artifacts of mine (the creature counts); theirs does not"
    );
}
