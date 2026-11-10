//! `cards/creatures/artifacts/mv_3/sliver_construct.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sliver_construct() -> CardIndex {
    card_index("7d592d5b-95cf-4416-9ade-1c23a8ba01d1")
}

/// Sliver Construct — {3}, 2/2 artifact creature, no rules text: it resolves
/// as that body with no keywords and offers no ability.
#[test]
fn sliver_construct_is_a_vanilla_two_two_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[sliver_construct()], &[forest(), forest(), forest()]);

    cast_and_settle(&mut engine, sliver_construct());

    let id = on_battlefield(&engine, p0, sliver_construct()).expect("it resolved");
    assert_eq!(pt(&engine, id), (2, 2));
    let t = types(&engine, id);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, id).is_empty());
    assert!(
        priority_offer(&engine)
            .abilities
            .iter()
            .all(|(s, _)| *s != id)
    );
}
