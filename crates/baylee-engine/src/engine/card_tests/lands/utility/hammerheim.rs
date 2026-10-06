//! `cards/lands/utility/hammerheim.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hammerheim: "{T}: Add {R}." / "{T}: Target creature loses all landwalk abilities until end of turn."
/// Under `Coverage::Partial`, the landwalk-removal ability is unsupported and omitted.
/// The legendary land taps to add {R} to the mana pool and offers no second ability.
#[test]
fn hammerheim_taps_for_red_mana_and_omits_landwalk_removal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[hammerheim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hammer = on_battlefield(&engine, p0, hammerheim()).expect("Hammerheim deployed");

    activate(&mut engine, p0, hammerheim(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, hammer));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == hammer && *ai == 1),
        "no second ability is offered"
    );
}
