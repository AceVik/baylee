//! `cards/lands/utility/urborg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urborg: "{T}: Add {B}." / "{T}: Target creature loses first strike or swampwalk until end of turn."
/// Under `Coverage::Partial`, the modal keyword loss ability is unsupported and omitted.
/// The legendary land taps to add {B} to the mana pool and offers no second ability.
#[test]
fn urborg_taps_for_black_mana_and_omits_keyword_loss() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(132, forest()).battlefield(0, &[urborg()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let u = on_battlefield(&engine, p0, urborg()).expect("Urborg deployed");

    activate(&mut engine, p0, urborg(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, u));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == u && *ai == 1),
        "no second ability is offered"
    );
}
