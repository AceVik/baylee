//! `cards/creatures/mv_7/birthing_hulk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn birthing_hulk() -> CardIndex {
    card_index("33899675-0de2-4adb-bf3f-55e18b493015")
}

/// Birthing Hulk — devoid, a 5/4 for {6}{G} that makes two 1/1 Eldrazi Scion
/// tokens when it enters. Devoid means it has no color, whatever its cost.
#[test]
fn birthing_hulk_is_colorless_and_makes_two_scions() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 7), &[birthing_hulk()], &[]);
    let hulk = cast_and_resolve(&mut engine, birthing_hulk());
    assert_body(&engine, hulk, (5, 4), KeywordSet::DEVOID);
    assert!(
        engine
            .state()
            .object(hulk)
            .expect("on the battlefield")
            .characteristics()
            .colors
            .is_empty(),
        "devoid: colorless although {{G}} is in the cost"
    );
    let scions = tokens_of(&engine, p0);
    assert_eq!(scions.len(), 2, "two Eldrazi Scion tokens");
    for scion in scions {
        assert_eq!(pt(&engine, scion), (1, 1));
    }
}

/// "{1}{C}: Regenerate this creature." Paid with a Sol Ring's {C}{C}, the
/// shield stands and replaces the destruction: the Hulk is still there,
/// tapped.
#[test]
fn birthing_hulk_regenerates_for_one_and_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[birthing_hulk(), sol_ring()], &[], &[]);
    let hulk = on_battlefield(&engine, p0, birthing_hulk()).expect("out");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, hulk, 1);
    kill(&mut engine, hulk);
    assert_eq!(
        on_battlefield(&engine, p0, birthing_hulk()),
        Some(hulk),
        "regenerated rather than destroyed"
    );
    assert!(is_tapped(&engine, hulk), "regeneration taps the creature");
}
