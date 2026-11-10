//! `cards/creatures/artifacts/mv_4/clay_statue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clay Statue — {4} — 3/1 artifact creature: "{2}: Regenerate this
/// creature."
///
/// Regeneration is not prevention and not an indestructible body: the
/// shield waits, and the next destruction this turn is replaced by "tap
/// it, remove all damage from it, and remove it from combat" (CR 701.19a).
/// The Statue is still on the battlefield after a destroy, tapped and with
/// the shield spent, where the same destroy takes an unshielded permanent
/// (CR 701.8a). The {2} is paid as the shield is bought, not when it is
/// spent.
#[test]
fn clay_statue_regenerates_out_of_a_destroy() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clay_statue(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let statue = on_battlefield(&engine, p0, clay_statue()).expect("seated");

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    raise_a_shield(&mut engine, p0, statue, 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} was the ability's price"
    );

    kill(&mut engine, statue);

    assert!(
        on_battlefield(&engine, p0, clay_statue()).is_some(),
        "the shield replaced the destruction (CR 701.19a)"
    );
    assert!(
        in_graveyard(&engine, p0, clay_statue()).is_none(),
        "so it never reached the graveyard"
    );
    assert_eq!(
        engine
            .state()
            .object(statue)
            .expect("still an object")
            .regeneration_shields,
        0,
        "and the shield was spent doing it"
    );
    assert!(
        is_tapped(&engine, statue),
        "regenerating taps what it saves (CR 701.19a)"
    );
}
