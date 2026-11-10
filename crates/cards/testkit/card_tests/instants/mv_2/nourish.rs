//! `cards/instants/mv_2/nourish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nourish is `{G}{G}` for one printed line: "You gain 6 life." The number is
/// the whole card, so the board is built so that six cannot be borrowed from
/// anywhere else — two Forests pay the cost and nothing else on the table can
/// move a life total — and the life is read on *both* seats, because a drain
/// and a gain are told apart by which side of the table moved. The spell is
/// read on the stack first (nothing is gained until it resolves) and in its
/// owner's graveyard afterwards, so the six life is the resolution of a spell
/// that actually was cast rather than a state somebody set.
#[test]
fn nourish_pays_two_green_for_six_life_on_its_casters_side_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[nourish()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The two Forests are the whole of `{G}{G}`; `cast_from_hand` taps them and
    // spends the pool in one step, so an empty pool afterwards is the printed
    // price really paid and not a free spell.
    cast_from_hand(&mut engine, p0, nourish());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "an instant on the stack hands priority back to its caster, got {:?}",
        engine.pending()
    );
    assert!(
        on_stack(&engine, nourish()).is_some(),
        "the spell is on the stack: an instant resolves when both seats pass"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing is gained until the spell resolves, so the life is read twice"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{G}} came out of the pool the two Forests filled"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        26,
        "\"You gain 6 life\" — six, and not two per Forest or one per mana"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the spell's controller: the opponent's total never moved"
    );
    assert!(
        in_graveyard(&engine, p0, nourish()).is_some(),
        "an instant that resolved goes to its owner's graveyard"
    );
}
