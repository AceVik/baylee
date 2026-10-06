//! `cards/creatures/mv_1/ivy_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ivy Elemental prints `{X}{G}` and "This creature enters with X +1/+1
/// counters on it" on a 0/0 body, so its printed power is never its power and
/// the counters are the whole card. Four Forests put exactly three mana
/// beyond the `{G}` in the pool, which is the X this test announces at
/// CR 601.2b — a three-counter 0/0 projects a 3/3, where a counter amount
/// frozen at one would read 1/1, and a card that forgot the counters would
/// read 0/0 and be eaten by CR 704.5f before the assertion. The drained pool
/// is the other half: a spell whose cost is `{X}{G}` cannot leave the four
/// mana it was cast with still floating.
#[test]
fn ivy_elemental_enters_with_its_announced_x_plus_one_plus_one_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[ivy_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        4,
        "four Forests, which is the {{G}} plus room for an X of three"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the mana is in the pool before the pool is read"
    );

    cast_with_floating(&mut engine, p0, ivy_elemental());
    let Pending::ChooseNumber { player, max, .. } = engine.pending().clone() else {
        panic!(
            "the value of X is announced while casting (CR 601.2b), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat casting the spell names its own X");
    // The resource bound includes every payable X; the final payment
    // below also accounts for the fixed green mana.
    assert!(
        max >= 3,
        "the announcement has to reach the three this board can pay: {max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(3))
        .expect("three is one of the values the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    let elemental =
        on_battlefield(&engine, p0, ivy_elemental()).expect("a 3/3 with three counters survives");
    assert_eq!(
        counters_on(&engine, elemental, CounterKind::P1P1),
        3,
        "\"enters with X +1/+1 counters on it\", and X was three"
    );
    assert_eq!(
        pt(&engine, elemental),
        (3, 3),
        "a printed 0/0 plus the three counters it entered with"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}} came out of the pool, X and all"
    );
}
