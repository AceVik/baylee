//! `cards/creatures/mv_7/macetail_hystrodon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "2f1e2742-d7df-4893-abc8-cb927c500569"

/// Macetail Hystrodon prints two halves that are only worth anything apart:
/// "{6}{R}" for a 4/4 Beast with first strike and haste, and "Cycling {3}".
/// The cycling line lives in hand and nowhere else, so it is played *before*
/// the card is ever cast — and it is played on the card that eats itself, so
/// the graveyard entry is the discard that paid the cost rather than a
/// creature that died. Ten Mountains pay for both inside one main phase
/// (CR 500.5), which makes the pool a ledger: ten, seven after the {3}, and
/// nothing once {6}{R} is spent on the copy still in hand.
#[test]
fn macetail_hystrodon_cycles_itself_and_then_lands_as_a_haste_first_striker() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 10])
        .hand(0, &[macetail_hystrodon(), macetail_hystrodon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Ten Mountains into the pool first: `can_afford` reads the pool and not
    // the untapped lands, so the cycling line's {3} is payable only from here.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        10,
        "ten Mountains, ten red, and the Hystrodon makes no mana of its own"
    );

    let cycled = in_hand(&engine, p0, macetail_hystrodon()).expect("a copy is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(cycled, 0)),
        "cycling is an ability of the card *in hand* and it is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, macetail_hystrodon(), 0);
    assert!(
        in_graveyard(&engine, p0, macetail_hystrodon()).is_some(),
        "\"discard this card\" is a cost, paid on announcement (CR 601.2h), so \
         the card is in the graveyard before the draw is anywhere"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the hand is the size it was: one card left as the cost and one arrived \
         as the effect, so a draw that never happened would read one short"
    );

    // The other copy, cast off the seven red the cycling left floating.
    // {6}{R} is exactly those seven, so the offer is read where the engine
    // reads it and the empty pool afterwards says the spell was really paid.
    let second = in_hand(&engine, p0, macetail_hystrodon()).expect("the other copy is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&second),
        "{{6}}{{R}} is affordable on the seven red still floating: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, macetail_hystrodon());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the seven floating red paid {{6}}{{R}} to the last mana"
    );
    let beast =
        on_battlefield(&engine, p0, macetail_hystrodon()).expect("the second copy resolved");
    assert_eq!(pt(&engine, beast), (4, 4), "the body the card prints");
    let granted = keywords(&engine, beast);
    assert!(granted.contains(KeywordSet::FIRST_STRIKE), "first strike");
    assert!(granted.contains(KeywordSet::HASTE), "haste");
    assert!(
        in_graveyard(&engine, p0, macetail_hystrodon()).is_some(),
        "and the cycled copy is still the card the discard put there"
    );
}
