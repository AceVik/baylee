//! `cards/creatures/mv_3/noble_panther.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Noble Panther — {1}{G}{W}, a 3/3 Cat — "{1}: This creature gains first
/// strike until end of turn."
///
/// The price is the mana and *not* the creature's own tap, so the board carries
/// four sources: they pay the {1}{G}{W} and leave exactly the {1} the ability
/// charges floating in the same main phase (CR 500.5). The offer is read after
/// that, because `legal.abilities` is filtered by `can_afford`, which reads the
/// pool and not the untapped lands — and read again once the pool is empty,
/// where the same untapped Panther is offered nothing, which is what tells a
/// mana price from a tap. The body staying a printed 3/3 through the activation
/// separates a keyword grant from a pump, and the second walk is the "until end
/// of turn" half.
#[test]
fn noble_panther_buys_first_strike_for_one_mana_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains(), plains(), plains()])
        .hand(0, &[noble_panther()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, noble_panther());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cat = on_battlefield(&engine, p0, noble_panther()).expect("the Panther resolved");
    assert_eq!(pt(&engine, cat), (3, 3), "the body the card prints");
    assert!(
        !keywords(&engine, cat).contains(KeywordSet::FIRST_STRIKE),
        "nothing has been bought yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{G}}{{W}} is spent and exactly the {{1}} the ability charges is left"
    );
    assert!(
        !is_tapped(&engine, cat),
        "the price is mana, not the creature's own {{T}}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cat, 0)),
        "with the {{1}} floating the Panther's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, noble_panther(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, cat).contains(KeywordSet::FIRST_STRIKE),
        "{{1}}: this creature gains first strike"
    );
    assert_eq!(
        pt(&engine, cat),
        (3, 3),
        "the ability grants a keyword and no body"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}} came out of the pool"
    );

    // The other half of the price, read with the Panther still untapped: an
    // empty pool pays no {1}, so the line is not offered — a cost that were
    // the tap would still be payable here.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cat, 0)),
        "nothing floating means no {{1}}, so there is nothing to activate: {:?}",
        legal.abilities
    );

    // "until end of turn": the grant is read once more with the turn behind
    // it. Nothing attacks, so the Panther is the only thing that can have
    // moved a keyword.
    pass_until(&mut engine, |e| {
        !keywords(e, cat).contains(KeywordSet::FIRST_STRIKE)
    });
    assert!(
        on_battlefield(&engine, p0, noble_panther()).is_some(),
        "the grant expired and the creature did not"
    );
}
