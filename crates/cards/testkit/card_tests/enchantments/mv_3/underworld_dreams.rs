//! `cards/enchantments/mv_3/underworld_dreams.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn underworld_dreams_card() -> CardIndex {
    card_index("967cf377-ae26-464d-85ac-8448b5a911f7")
}

/// Underworld Dreams at a table of four, each seat's draw step watched on
/// its own: the enchantment hits the opponent who drew and nobody else.
///
/// Heads-up the opponent who drew and "each opponent" are one seat, so a
/// card that damaged every opponent, or only the lowest-numbered one, would
/// pass a duel. Here seat 2's draw must cost seat 2 and leave seats 1 and 3
/// alone, and the controller's own draw must change no life at all.
#[test]
fn underworld_dreams_pings_only_the_opponent_who_drew() {
    let seat = PlayerId::new;
    let lives = |e: &Engine<RegistryLookup>| -> Vec<i32> {
        e.state().players.iter().map(|p| p.life).collect()
    };
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[underworld_dreams_card()])
        .start();
    keep_mulligans(&mut engine);

    // Seat 1's draw: seat 1 alone takes 1.
    let start = lives(&engine);
    reach_their_main_phase(&mut engine, seat(1));
    pass_until(&mut engine, stack_is_empty);
    let after_one = lives(&engine);
    assert_eq!(
        after_one,
        [start[0], start[1] - 1, start[2], start[3]],
        "seat 1 drew: seat 1 takes 1 and nobody else does"
    );

    // Seat 2's draw is the one that is not the first opponent in seat order.
    reach_their_main_phase(&mut engine, seat(2));
    pass_until(&mut engine, stack_is_empty);
    let after_two = lives(&engine);
    assert_eq!(
        after_two,
        [after_one[0], after_one[1], after_one[2] - 1, after_one[3]],
        "seat 2 drew: seat 2 takes 1, seat 1 nothing more, seat 3 nothing"
    );

    // Seat 3, then the controller's own draw step: that one does nothing.
    reach_their_main_phase(&mut engine, seat(3));
    pass_until(&mut engine, stack_is_empty);
    let after_three = lives(&engine);
    assert_eq!(
        after_three,
        [after_two[0], after_two[1], after_two[2], after_two[3] - 1],
        "seat 3 drew: seat 3 takes 1 and seat 2 nothing more"
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(seat(0))).len();
    reach_their_main_phase(&mut engine, seat(0));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(seat(0))).len(),
        hand_before + 1,
        "the controller did draw"
    );
    assert_eq!(
        lives(&engine),
        after_three,
        "the controller's own draw is not an opponent's: no life changes at the table"
    );
}
