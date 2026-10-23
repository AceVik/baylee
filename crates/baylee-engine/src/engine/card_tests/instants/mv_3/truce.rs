//! Truce — `{2}{W}` Instant (Homelands)
//!
//! Each player may draw up to two cards. For each card less than two a player draws this way,
//! that player gains 2 life.
//!
//! The card is `Partial`: each of the caster and (one) opponent may draw two cards or none, and the
//! life for the cards not drawn is not gained.

#[allow(clippy::wildcard_imports)]
use super::*;

fn truce() -> CardIndex {
    card_index("fe4aa72b-f176-461a-aca0-1519721609b2")
}

/// P0 casts Truce; P0 answers `caster_draws` and P1 answers `opponent_draws` to "may draw". Returns
/// the change in P0's hand size (the Truce itself left it), the change in P1's hand size, and both
/// life totals.
fn cast_truce(caster_draws: bool, opponent_draws: bool) -> (isize, isize, [i32; 2]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[truce()])
        .battlefield(0, &[plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let truce_card = in_hand(&engine, p0, truce()).unwrap();
    let hand_len = |e: &Engine<RegistryLookup>, seat| {
        isize::try_from(e.state().zones.list(ZoneLocation::Hand(seat)).len()).unwrap()
    };
    let (p0_before, p1_before) = (hand_len(&engine, p0), hand_len(&engine, p1));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: truce_card })
        .unwrap();
    // "Each player may": the caster is asked, then the opponent.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the caster decides first");
    engine.apply(p0, PlayerAction::YesNo(caster_draws)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p1, "the opponent decides for themselves");
    engine
        .apply(p1, PlayerAction::YesNo(opponent_draws))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    (
        hand_len(&engine, p0) - p0_before,
        hand_len(&engine, p1) - p1_before,
        [
            engine.state().players[0].life,
            engine.state().players[1].life,
        ],
    )
}

#[test]
fn truce_lets_each_player_draw_two_cards() {
    // The caster's Truce left the hand (-1) and two cards came in; the opponent drew two.
    assert_eq!(cast_truce(true, true), (1, 2, [20, 20]));
}

#[test]
fn truce_does_not_make_a_player_draw_who_declines() {
    // Declining is each player's own choice: the other still draws. (The 2 life per undrawn card
    // is not on the card, so a decliner gains nothing.)
    assert_eq!(cast_truce(false, true), (-1, 2, [20, 20]));
    assert_eq!(cast_truce(true, false), (1, 0, [20, 20]));
    assert_eq!(cast_truce(false, false), (-1, 0, [20, 20]));
}
