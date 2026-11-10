//! `cards/sorceries/mv_1/rite_of_flame.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rite of Flame ({R} sorcery): "Add {R}{R}, …".
///
/// A ritual is the one spell whose whole effect is a number, and a number is
/// exactly what reading the card cannot check: `Effect::mana(ManaColor::Red,
/// 2)` says two red and says nothing about the `{R}` that was paid for it, so
/// a card that added its mana without ever taking the cost, or took the cost
/// and added it to the wrong colour, reads identically. The pool is measured
/// at three moments for that reason — one red floating off the Mountain, an
/// **empty** pool the instant the spell is on the stack, and two red when it
/// has resolved — and the middle one is the half a test that only looked at
/// the end would miss.
///
/// The board is one Mountain and nothing else, and `legal.mana_abilities` is
/// asserted empty once it is tapped: every red counted afterwards came out of
/// the spell, because there is nothing left on the table that could make one.
///
/// Then whether it is *mana* rather than a number in a struct. `{1}{R}` is
/// not castable on the single red the Mountain made and is castable on what
/// the Rite leaves behind, at the same priority, off the same tapped board —
/// so the two the pool reports are two the engine will let a spell spend.
///
/// The second printed clause — "then add {R} for each card named Rite of
/// Flame in each graveyard" — has nothing to count here: no graveyard holds
/// a Rite of Flame while this one resolves (the card is on the stack, and
/// nothing else was cast). The next test is about that clause.
#[test]
fn rite_of_flame_spends_one_red_and_leaves_two_spendable_in_the_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[rite_of_flame(), a_two_mana_red_spell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Mountain is tapped"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain's own {{R}}, which is all the mana this board has"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.is_empty(),
        "the Mountain is tapped and there is no second land: nothing on this \
         board can add mana any more, so every red counted below came out of \
         the spell — {legal:?}"
    );
    let rite = in_hand(&engine, p0, rite_of_flame()).expect("the Rite is in hand");
    let two_drop = in_hand(&engine, p0, a_two_mana_red_spell()).expect("the two-drop is in hand");
    assert!(
        legal.castable.contains(&rite),
        "one floating red pays the printed {{R}}: {legal:?}"
    );
    assert!(
        !legal.castable.contains(&two_drop),
        "and does not pay {{1}}{{R}} — the before half of the offer this test \
         reads again after the Rite has resolved: {legal:?}"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: rite })
        .expect("the spell the offer just named");
    assert!(
        !stack_is_empty(&engine),
        "a sorcery goes on the stack (CR 601.2a)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Mountain's {{R}} was spent on the cost: a ritual that added \
         without ever taking payment would look the same at the end and is \
         only distinguishable here"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    let pool = &engine.state().players[0].mana_pool;
    for color in ManaColor::ALL {
        let expected: u32 = if color == ManaColor::Red { 2 } else { 0 };
        assert_eq!(
            pool.available(color),
            expected,
            "`Add {{R}}{{R}}` put two red in a pool the cast had just emptied \
             and touched no other colour — {color:?} disagrees"
        );
    }
    assert_eq!(
        pool.total(),
        2,
        "two and no third: nothing restricted was added beside the plain \
         red, and the clause about cards named Rite of Flame in graveyards \
         has nothing to count while the only copy is the one resolving"
    );
    assert!(
        in_graveyard(&engine, p0, rite_of_flame()).is_some(),
        "a resolved sorcery is put into its owner's graveyard (CR 608.2n)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&two_drop),
        "the same {{1}}{{R}} card that was unaffordable one priority ago is \
         castable now, off the same tapped Mountain: what the Rite added is \
         mana a spell can be paid with and not a number in a pool: {legal:?}"
    );
}

/// Rite of Flame's second clause: "then add {R} for each card named Rite of
/// Flame in each graveyard."
///
/// One copy in p0's graveyard and two in p1's, so both words are read:
/// "each graveyard" counts across the table, where a count of your own would
/// add three. The five is also the answer to whether the resolving copy
/// counts itself — it is on the stack until the very end of its resolution
/// (CR 608.2n), so a sixth would mean it had been counted from the
/// graveyard.
#[test]
fn rite_of_flame_adds_one_more_red_for_each_copy_in_each_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The library filler is the card itself, which is how copies reach a
    // graveyard nobody cast them from.
    let mut engine = Duel::new(83, rite_of_flame())
        .battlefield(0, &[mountain()])
        .hand(0, &[rite_of_flame()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 2);

    cast_from_hand(&mut engine, p0, rite_of_flame());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        5,
        "{{R}}{{R}}, then one for each of the three copies across both graveyards"
    );
    assert_eq!(pool.total(), 5, "and no other mana");
}
