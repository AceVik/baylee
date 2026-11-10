//! `cards/creatures/artifacts/mv_2/darklit_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn darklit_gargoyle() -> CardIndex {
    card_index("e48d9efa-c169-4587-86fc-0021f265d69b")
}

/// Darklit Gargoyle: a 1/2 with "Flying": only a flier may block it.
#[test]
fn darklit_gargoyle_has_flying() {
    only_a_flier_may_block(darklit_gargoyle());
}

/// "{B}: This creature gets +2/-1 until end of turn": one black mana makes it
/// a 3/1; a second activation makes toughness 0 and it dies.
#[test]
fn darklit_gargoyle_gets_plus_two_minus_one_per_black() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[darklit_gargoyle(), swamp(), swamp()], &[]);
    let gargoyle = on_battlefield(&engine, p0, darklit_gargoyle()).expect("seated");
    assert_eq!(pt(&engine, gargoyle), (1, 2));
    assert!(
        !priority_offer(&engine).abilities.contains(&(gargoyle, 0)),
        "no black mana floating"
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, darklit_gargoyle(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, gargoyle), (3, 1), "+2/-1");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one {{B}} paid"
    );

    activate(&mut engine, p0, darklit_gargoyle(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, darklit_gargoyle()).is_none(),
        "toughness 0: it died"
    );
    assert!(in_graveyard(&engine, p0, darklit_gargoyle()).is_some());
}

/// A pumped Gargoyle attacks for the new power.
#[test]
fn darklit_gargoyle_pumped_hits_for_three() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[darklit_gargoyle(), swamp()], &[]);
    let gargoyle = on_battlefield(&engine, p0, darklit_gargoyle()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, darklit_gargoyle(), 0);
    pass_until(&mut engine, stack_is_empty);
    swing_unblocked(&mut engine, gargoyle);
    assert_eq!(life_of(&engine, 1), 17);
}
