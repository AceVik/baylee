//! `cards/artifacts/equipment/mv_2/gilded_pinions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gilded_pinions() -> CardIndex {
    card_index("8a7c3718-74a1-4f26-8e06-0e17db4b0077")
}

/// Gilded Pinions: "Equipped creature has flying. Equip {2}".
#[test]
fn gilded_pinions_give_flying() {
    equipped_gets(
        gilded_pinions(),
        2,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FLYING,
    );
}

/// "When this Equipment enters, create a Treasure token."
#[test]
fn gilded_pinions_entering_create_a_treasure() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gilded_pinions()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no token before the cast"
    );
    cast_from_hand(&mut engine, p0, gilded_pinions());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "exactly one token");
    assert!(types(&engine, tokens[0]).contains(TypeSet::ARTIFACT));
    assert!(
        subtypes_of(&engine, tokens[0])
            .contains(baylee_core::generated::subtypes::artifact::TREASURE),
        "it is a Treasure"
    );
    assert!(on_battlefield(&engine, p0, gilded_pinions()).is_some());
}
