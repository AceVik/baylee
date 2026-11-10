//! `cards/instants/mv_2/clear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clear — {1}{W} Instant: "Destroy target enchantment."
///
/// The whole spell is that one sentence, so the board is built to make the
/// printed word the only thing that can decide the outcome: one enchantment
/// stands *across* the table and one Forest stands beside it. The Forest is
/// the witness — a filter widened to "target permanent" would offer it, and
/// one narrowed to "enchantment you control" would offer nothing at all — and
/// the card's own graveyard entry is what separates destruction from an exile
/// or a bounce that also took the permanent off the battlefield.
#[test]
fn clear_destroys_the_enchantment_across_the_table_and_nothing_beside_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[clear()])
        .battlefield(1, &[their_enchantment(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doomed = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let witness = on_battlefield(&engine, p1, forest()).expect("and a Forest beside it");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the cast: the two Plains are the whole price"
    );

    // Two Plains pay {1}{W} and nothing else on this board can — the Forest
    // belongs to the other seat — so the pool read afterwards is the printed
    // cost and not a stray source.
    cast_from_hand(&mut engine, p0, clear());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster aims their own spell");
    assert_eq!((min, max), (1, 1), "\"target enchantment\" is exactly one");
    assert!(
        options.contains(&doomed),
        "\"target enchantment\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&witness),
        "a Forest is a permanent and no enchantment: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and those two readings are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the enchantment the question offered is the one it destroys");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, clear()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} came out of the pool the two Plains filled"
    );
}
