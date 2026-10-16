//! `cards/creatures/artifacts/mv_3/stonework_puma.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn stonework_puma() -> CardIndex {
    card_index("841b1010-a7d3-415e-b76d-7b50fea909cb")
}

/// Stonework Puma — {3}, 2/2 artifact creature, no rules text.
#[test]
fn stonework_puma_is_a_vanilla_two_two_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[stonework_puma()], &[forest(), forest(), forest()]);

    cast_and_settle(&mut engine, stonework_puma());

    let id = on_battlefield(&engine, p0, stonework_puma()).expect("it resolved");
    assert_eq!(pt(&engine, id), (2, 2));
    let t = types(&engine, id);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, id).is_empty());
    assert!(priority_offer(&engine).abilities.iter().all(|(s, _)| *s != id));
}
