//! `cards/creatures/mv_4/teroh_s_faithful.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Teroh's Faithful is a {3}{W} 1/4 Human Cleric whose whole printed text is
/// "When this creature enters, you gain 4 life", so the scenario has to show
/// both halves of that sentence and nothing else: the body arriving as printed,
/// and exactly one seat's life total moving by exactly four. The spell is read
/// on the stack first — both seats still at twenty, so the gain is the *entry*
/// trigger and not the cast — and the pool is empty and the board bare of any
/// other lifegain source, which is what makes 24 and 20 a statement about the
/// card rather than about the board it landed on.
#[test]
fn terohs_faithful_gains_four_life_for_the_seat_it_entered_under() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[terohs_faithful()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `can_afford` reads the pool and not the untapped lands, so the {3}{W} is
    // a real price: with nothing floating the Cleric is not castable at all.
    let card = in_hand(&engine, p0, terohs_faithful()).expect("the Cleric is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds a quiet main phase, got {:?}",
            engine.pending()
        );
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{W}}, so the Cleric is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains tapped, four white"
    );
    cast_with_floating(&mut engine, p0, terohs_faithful());
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "the spell is still on the stack: nothing has entered and nothing has \
         been gained"
    );

    pass_until(&mut engine, stack_is_empty);

    let faithful = on_battlefield(&engine, p0, terohs_faithful()).expect("the Cleric resolved");
    assert_eq!(
        pt(&engine, faithful),
        (1, 4),
        "the printed body, and no pump under it"
    );
    assert!(
        types(&engine, faithful).contains(TypeSet::CREATURE),
        "it arrived as a creature and not merely as a card that changed zones"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"When this creature enters, you gain 4 life\" — four, exactly once"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the controller: the seat across the table gained nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{W}} came out of the pool"
    );
}
