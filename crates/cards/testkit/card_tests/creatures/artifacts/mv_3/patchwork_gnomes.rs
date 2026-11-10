//! `cards/creatures/artifacts/mv_3/patchwork_gnomes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn patchwork_gnomes() -> CardIndex {
    card_index("00ad27a1-9162-408d-ac75-970e45d7e06c")
}

/// Patchwork Gnomes — "Discard a card: Regenerate this creature." Paid with a
/// Forest from hand, the Gnomes then survive being destroyed, tapped.
#[test]
fn patchwork_gnomes_regenerates_for_a_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[forest()], &[patchwork_gnomes()]);
    let gnomes = on_battlefield(&engine, p0, patchwork_gnomes()).expect("seated");
    assert_eq!(pt(&engine, gnomes), (2, 1));

    activate(&mut engine, p0, patchwork_gnomes(), 0);
    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(in_graveyard(&engine, p0, forest()).is_some(), "the discard");
    assert_eq!(
        engine.state().object(gnomes).unwrap().regeneration_shields,
        1,
        "one shield stands"
    );

    kill(&mut engine, gnomes);
    assert!(
        on_battlefield(&engine, p0, patchwork_gnomes()).is_some(),
        "destroyed, it regenerated instead"
    );
    assert!(is_tapped(&engine, gnomes), "regeneration taps it");
}

/// Without the discard there is no shield and destruction is final.
#[test]
fn patchwork_gnomes_dies_without_a_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[forest()], &[patchwork_gnomes()]);
    let gnomes = on_battlefield(&engine, p0, patchwork_gnomes()).expect("seated");

    kill(&mut engine, gnomes);

    assert!(on_battlefield(&engine, p0, patchwork_gnomes()).is_none());
    assert!(in_graveyard(&engine, p0, patchwork_gnomes()).is_some());
}
