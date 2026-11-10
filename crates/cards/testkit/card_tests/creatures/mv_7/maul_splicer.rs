//! `cards/creatures/mv_7/maul_splicer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::subtypes;

fn maul_splicer() -> CardIndex {
    card_index("b0d0c534-8bd2-4801-aadd-dcc9c92d6ec3")
}

/// Maul Splicer — a 1/1 for {6}{G} that makes two 3/3 colorless Phyrexian
/// Golem artifact creature tokens, and "Golem creatures you control have
/// trample": the Golems do, the Splicer (a Human Artificer) does not.
#[test]
fn maul_splicer_makes_two_trampling_golems() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 7), &[maul_splicer()], &[]);
    let splicer = cast_and_resolve(&mut engine, maul_splicer());
    assert_eq!(pt(&engine, splicer), (1, 1));
    assert!(!keywords(&engine, splicer).contains(KeywordSet::TRAMPLE));
    let golems = tokens_of(&engine, p0);
    assert_eq!(golems.len(), 2);
    for golem in golems {
        assert_eq!(pt(&engine, golem), (3, 3));
        assert!(keywords(&engine, golem).contains(KeywordSet::TRAMPLE));
        let c = engine.state().object(golem).expect("out").characteristics();
        assert!(c.types.contains(TypeSet::ARTIFACT) && c.types.contains(TypeSet::CREATURE));
        assert!(c.subtypes.contains(subtypes::creature::GOLEM));
        assert!(c.colors.is_empty(), "colorless");
    }
}
