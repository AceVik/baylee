//! `cards/creatures/artifacts/mv_3/wicker_witch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wicker_witch() -> CardIndex {
    card_index("cc2f945f-9048-4978-b61f-49eefb3d94d7")
}

/// Wicker Witch — {3}, 3/1 artifact creature, no rules text.
#[test]
fn wicker_witch_is_a_vanilla_three_one_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[wicker_witch()], &[forest(), forest(), forest()]);

    cast_and_settle(&mut engine, wicker_witch());

    let id = on_battlefield(&engine, p0, wicker_witch()).expect("it resolved");
    assert_eq!(pt(&engine, id), (3, 1));
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
