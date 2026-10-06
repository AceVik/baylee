//! `cards/sorceries/mv_4/ambition_s_cost.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ambition's Cost prints one sentence with two halves: "You draw three cards
/// and you lose 3 life." Neither half is the other's control, so the scenario
/// plays the card for real off exactly four Swamps — `{3}{B}` and nothing
/// spare — and reads the three cards on the library *and* the hand together,
/// so an emptied library could not stand in for a draw. The 3 life is claimed
/// on both seats, because "you lose" is the caster's word and a card that had
/// billed the opponent would leave p0 at twenty; and the pool reads empty
/// afterwards, so the price was paid rather than merely printed.
#[test]
fn ambition_s_cost_draws_three_cards_and_bills_its_own_controller_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[ambition_s_cost()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Swamps are exactly `{3}{B}`, so the pool is a real payment: the
    // offer is read with the mana already floating, because `can_afford`
    // reads the pool and not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, four black — the whole printed cost and nothing over"
    );
    let card = in_hand(&engine, p0, ambition_s_cost()).expect("the sorcery is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "the printed cost is payable in the pool, so the sorcery is offered: {:?}",
        legal.castable
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p0, ambition_s_cost());
    assert!(
        !stack_is_empty(&engine),
        "a sorcery uses the stack, unlike the mana ability that paid for it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "both halves of the sentence resolve together, so nothing has happened yet"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "\"You draw three cards\": three off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "the sorcery left the hand and three cards arrived, so an emptied \
         library could not satisfy the count above"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"and you lose 3 life\" — three, and never three points per card drawn"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that cast the spell, not to the opponent"
    );
    assert!(
        in_graveyard(&engine, p0, ambition_s_cost()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the four Swamps' mana was the price, spent to the last point"
    );
}
