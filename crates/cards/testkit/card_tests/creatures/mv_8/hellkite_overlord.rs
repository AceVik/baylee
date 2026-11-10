//! `cards/creatures/mv_8/hellkite_overlord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hellkite_overlord() -> CardIndex {
    card_index("1d05e441-84d4-482f-95a8-7842e776cad2")
}

/// Hellkite Overlord is an 8/8 for {4}{B}{R}{R}{G} with flying, trample and
/// haste.
#[test]
fn hellkite_overlord_is_cast_for_nine_with_flying_trample_and_haste() {
    let mut engine = at_main(
        &[
            swamp(),
            mountain(),
            mountain(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
        ],
        &[hellkite_overlord()],
    );
    let dragon = cast_and_settle(&mut engine, hellkite_overlord());
    assert_eq!(pt(&engine, dragon), (8, 8));
    let k = keywords(&engine, dragon);
    assert!(k.contains(KeywordSet::FLYING));
    assert!(k.contains(KeywordSet::TRAMPLE));
    assert!(k.contains(KeywordSet::HASTE));
}

/// "{R}: This creature gets +1/+0 until end of turn."
#[test]
fn hellkite_overlord_firebreathes_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[hellkite_overlord(), mountain(), mountain()], &[]);
    let dragon = on_battlefield(&engine, p0, hellkite_overlord()).expect("the Overlord");
    assert_eq!(pt(&engine, dragon), (8, 8));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, hellkite_overlord(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (9, 8), "+1/+0 for one {{R}}");
    activate(&mut engine, p0, hellkite_overlord(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (10, 8), "and again for the second");
}

/// "{B}{G}: Regenerate this creature": with a shield bought, a destroy
/// effect leaves the Overlord on the battlefield, tapped and with the shield
/// spent; without one the same effect puts it in the graveyard.
#[test]
fn hellkite_overlord_regenerates_for_black_and_green() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[hellkite_overlord(), swamp(), forest()], &[]);
    let dragon = on_battlefield(&engine, p0, hellkite_overlord()).expect("the Overlord");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, dragon, 1);
    kill(&mut engine, dragon);
    assert!(
        on_battlefield(&engine, p0, hellkite_overlord()).is_some(),
        "regenerated instead of dying"
    );
    assert!(is_tapped(&engine, dragon), "regeneration taps it");
    assert_eq!(
        engine.state().object(dragon).unwrap().regeneration_shields,
        0,
        "the shield is used up"
    );

    let mut bare = at_main(&[hellkite_overlord()], &[]);
    let dragon = on_battlefield(&bare, p0, hellkite_overlord()).expect("the Overlord");
    kill(&mut bare, dragon);
    assert!(
        in_graveyard(&bare, p0, hellkite_overlord()).is_some(),
        "no shield, no regeneration"
    );
}
