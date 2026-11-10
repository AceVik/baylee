//! `cards/creatures/artifacts/mv_1/universal_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn universal_automaton() -> CardIndex {
    card_index("d50b1d2e-1825-432a-ab73-068fdc356f50")
}

/// "Changeling": the projected subtypes hold creature types the card never
/// prints, from both ends of the alphabet.
#[test]
fn universal_automaton_is_every_creature_type() {
    let p0 = PlayerId::new(0);
    let engine = main_with(&[universal_automaton()], &[]);
    let me = on_battlefield(&engine, p0, universal_automaton()).expect("seated");
    assert!(keywords(&engine, me).contains(KeywordSet::CHANGELING));
    let subtypes = engine
        .state()
        .object(me)
        .expect("object")
        .characteristics()
        .subtypes;
    for ty in [
        baylee_core::generated::subtypes::creature::GOBLIN,
        baylee_core::generated::subtypes::creature::ELF,
        baylee_core::generated::subtypes::creature::SHAPESHIFTER,
        baylee_core::generated::subtypes::creature::ZOMBIE,
    ] {
        assert!(subtypes.contains(ty), "{ty:?}");
    }
    assert_eq!(pt(&engine, me), (1, 1));
}
