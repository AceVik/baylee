//! `cards/creatures/mv_3/fire_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fire Drake is `{1}{R}{R}` for a flying 1/2 with `{R}: This creature gets
/// +1/+0 until end of turn. Activate only once each turn.` Both printed halves
/// need the card to actually be played: the flying is only readable once the
/// permanent is on the battlefield and projected, and the pump is the only
/// thing that ever moves its power off the printed 1. Five Mountains pay the
/// spell and leave two red floating, so the pool still holds a red when the
/// second claim is made — which is what makes the ability's absence after one
/// activation the once-each-turn clause rather than an unpayable `{R}`.
#[test]
fn fire_drake_pumps_once_a_turn_while_the_pool_still_has_the_red_to_pay() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[fire_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fire_drake());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, fire_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (1, 2), "the body the card prints");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five Mountains paid {{1}}{{R}}{{R}} and left two red"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the
    // pool, so the offer is only worth reading with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(drake, 0)),
        "a red is floating, so the pump is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, fire_drake(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining +1/+0 is no mana ability, so the pump uses the stack"
    );
    assert_eq!(
        pt(&engine, drake),
        (1, 2),
        "and has not resolved yet: the body is still the printed one"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, drake),
        (2, 2),
        "\"{{R}}: This creature gets +1/+0 until end of turn\""
    );

    // The limit, and its control: one red is still floating, so the ability
    // is affordable and its absence is the once-each-turn clause and not an
    // empty pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the red the second activation would need is still there"
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == drake),
        "\"Activate only once each turn\" — the pump is gone from the offer \
         with the mana still floating: {:?}",
        legal.abilities
    );
}
