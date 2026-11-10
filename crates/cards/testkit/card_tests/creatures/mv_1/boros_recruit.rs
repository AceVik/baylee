//! `cards/creatures/mv_1/boros_recruit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boros Recruit is a 1/1 Goblin Soldier with first strike for `{R/W}`, and
/// the hybrid symbol is the entire card: one mana, payable with either half.
/// The test casts one copy off a Mountain while the Plains stands untouched —
/// the untapped land is what says which half paid — and then a second copy
/// off that Plains alone, which the red half cannot pay for. Reading the
/// bodies afterwards is the other half of the claim: a symbol that resolved
/// into a creature with no first strike would satisfy every mana assertion
/// above it.
#[test]
fn boros_recruit_is_paid_by_either_half_of_its_hybrid_symbol_and_strikes_first() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), plains()])
        .hand(0, &[boros_recruit(), boros_recruit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hill = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    let field = on_battlefield(&engine, p0, plains()).expect("the Plains is out");

    // `{R}`: the Mountain taps and the Plains is the one source named as kept
    // back, so the mana in the pool has no other source on this board.
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    assert!(is_tapped(&engine, hill), "the red source paid");
    assert!(
        !is_tapped(&engine, field),
        "and the white one is still standing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let first = in_hand(&engine, p0, boros_recruit()).expect("a Recruit is in hand");
    assert!(
        legal.castable.contains(&first),
        "one red mana is the whole of {{R/W}}: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, boros_recruit());
    pass_until(&mut engine, |e| at_rest(e, p0));

    // `{W}`: the same symbol off the other half, with the Mountain now the
    // source that stays down.
    tap_all_mana_but(&mut engine, p0, Some(mountain()));
    assert!(is_tapped(&engine, field), "the Plains tapped for white");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let second = in_hand(&engine, p0, boros_recruit()).expect("the second Recruit is in hand");
    assert!(
        legal.castable.contains(&second),
        "and one white mana is the same symbol: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, boros_recruit());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let recruits = all_on_battlefield(&engine, p0, boros_recruit());
    assert_eq!(
        recruits.len(),
        2,
        "both halves of the symbol paid for a body"
    );
    assert_eq!(pt(&engine, recruits[0]), (1, 1), "a printed 1/1");
    assert!(
        keywords(&engine, recruits[0]).contains(KeywordSet::FIRST_STRIKE),
        "and first strike, the only line of text the card has"
    );
}
