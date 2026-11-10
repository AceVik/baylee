//! `cards/creatures/mv_6/flowstone_mauler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Mauler is "{4}{R}{R}" for a 4/5 Beast with trample, plus
/// "{R}: This creature gets +1/-1 until end of turn" — an activated ability
/// with no once-per-turn limit, which is the whole card. Eleven Mountains pay
/// the cast and leave exactly five red, and the five activations they buy are
/// read one resolved step at a time: the body walks 4/5 → 5/4 → … → 8/1 while
/// the pool drops by one each time, so "the pump stacks" and "the {R} was
/// really paid" are the same ladder. The fifth rung is the printed toughness
/// running out — a 9/0 is lethal without a single point of damage (CR 704.5f).
#[test]
fn flowstone_mauler_stacks_its_own_pump_until_its_toughness_runs_out() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 11])
        .hand(0, &[flowstone_mauler()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // Eleven tapped Mountains are the pool: the cast's {4}{R}{R} comes out of
    // it and the five red the ability charges are what is left, since a pool
    // survives until the step ends (CR 500.5) and this all happens in one.
    cast_from_hand(&mut engine, p0, flowstone_mauler());
    pass_until(&mut engine, stack_is_empty);
    let mauler = on_battlefield(&engine, p0, flowstone_mauler()).expect("the Beast resolved");
    assert_eq!(pt(&engine, mauler), (4, 5), "the body the card prints");
    assert!(
        keywords(&engine, mauler).contains(KeywordSet::TRAMPLE),
        "and the printed trample reaches the permanent"
    );

    let mut floating = engine.state().players[0].mana_pool.total();
    assert_eq!(
        floating, 5,
        "eleven Mountains less the {{4}}{{R}}{{R}} the cast cost"
    );

    // Each activation is resolved before the next is read: a pump the engine
    // has only announced is not on the body yet.
    let ladder: [(i16, i16); 4] = [(5, 4), (6, 3), (7, 2), (8, 1)];
    for (step, body) in ladder.into_iter().enumerate() {
        activate(&mut engine, p0, flowstone_mauler(), 0);
        assert!(
            !stack_is_empty(&engine),
            "pumping is no mana ability, so activation {} waits on the stack",
            step + 1
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            pt(&engine, mauler),
            body,
            "activation {} is a +1/-1 on top of every one before it",
            step + 1
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            floating - 1,
            "and its {{R}} came out of the pool"
        );
        floating -= 1;
    }

    // The fifth one is the printed toughness leaving: a 5 with five -1s is a
    // 0, and no attack, damage or destroy effect was needed to get there.
    assert_eq!(pt(&engine, mauler), (8, 1), "one -1 away from nothing");
    activate(&mut engine, p0, flowstone_mauler(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, flowstone_mauler()).is_none(),
        "the Beast pumped itself to zero toughness and the state-based action \
         took it (CR 704.5f)"
    );
    assert!(
        in_graveyard(&engine, p0, flowstone_mauler()).is_some(),
        "and it is in its owner's graveyard, not merely gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five activations took exactly the five red the cast left"
    );
}
