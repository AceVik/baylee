//! `cards/creatures/artifacts/mv_3/redrock_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn redrock_sentinel() -> CardIndex {
    card_index("51a6819d-c8fb-4f52-ab0c-4ae845728ed2")
}

/// Redrock Sentinel — defender 2/4; "{2}, {T}, Sacrifice a land: Draw a card
/// and create a Treasure token."
#[test]
fn redrock_sentinel_sacrifices_a_land_to_draw_and_make_a_treasure() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[redrock_sentinel(), forest(), forest(), forest()]);
    let sentinel = on_battlefield(&engine, p0, redrock_sentinel()).expect("seated");
    assert_eq!(pt(&engine, sentinel), (2, 4));
    assert!(keywords(&engine, sentinel).contains(KeywordSet::DEFENDER));
    let library_before = library_of(&engine, p0).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, redrock_sentinel(), 0);
    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "one land sacrificed"
    );
    assert_eq!(library_of(&engine, p0).len(), library_before - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "a Treasure");
    assert!(types(&engine, tokens[0]).contains(TypeSet::ARTIFACT));
    assert!(is_tapped(&engine, sentinel), "{{T}} is a cost");
}
