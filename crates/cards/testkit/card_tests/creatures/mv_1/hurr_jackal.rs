//! `cards/creatures/mv_1/hurr_jackal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hurr Jackal — `{R}` 1/1 Jackal: "{T}: Target creature can't be
/// regenerated this turn."
///
/// A regeneration shield stands on Lotleth Troll before the activation
/// (bought off the Troll's own `{B}` ability, so the test has read the
/// regeneration rule as well as this one), and the Troll dies anyway:
/// CR 701.19c, the shield is not applied. The shield's presence is asserted
/// before the kill, because a board without one would let a mutant survive.
#[test]
fn hurr_jackal_stops_a_shielded_creature_from_regenerating() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hurr_jackal(), lotleth_troll(), mountain(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let jackal = on_battlefield(&engine, p0, hurr_jackal()).expect("seated");
    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("seated");
    assert_eq!(pt(&engine, jackal), (1, 1), "the printed body");

    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);
    assert_eq!(
        engine
            .state()
            .object(troll)
            .expect("the shielded Troll is still there")
            .regeneration_shields,
        1,
        "one shield, standing, before the Jackal says otherwise"
    );

    activate(&mut engine, p0, hurr_jackal(), 0);
    aim_at(&mut engine, p0, troll);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(is_tapped(&engine, jackal), "the {{T}} was the price");

    kill(&mut engine, troll);
    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "\"can't be regenerated this turn\": the shield stood and was not \
         applied (CR 701.19c)"
    );
    assert!(
        on_battlefield(&engine, p0, lotleth_troll()).is_none(),
        "a creature that dies leaves the battlefield"
    );
}
