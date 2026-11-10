//! `cards/creatures/mv_1/phyrexian_battleflies.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Battleflies is a `{B}` 0/1 flier whose only printed line is
/// "`{B}`: This creature gets +1/+0 until end of turn. Activate no more than
/// twice each turn." The cap is the whole card, so the scenario reads it from
/// three sides at once: two activations climb a 0/1 to a 2/1, the third is
/// refused **while a black mana is still floating** — the cap and not the
/// price is what withholds it — and both the pump and the cap turn over with
/// the turn, which is the difference between "twice each turn" and "twice".
#[test]
fn phyrexian_battleflies_pumps_twice_a_turn_and_no_third_time() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(812, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_battleflies()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Swamps tapped up front: one pays the creature's {B} and the other
    // three are the two pumps the card allows plus the one it does not, so
    // the refusal below can never be a refusal for want of mana (CR 601.2h
    // reads the pool, not the untapped lands).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4,
        "four Swamps, four black"
    );
    cast_with_floating(&mut engine, p0, phyrexian_battleflies());
    pass_until(&mut engine, stack_is_empty);

    let flies =
        on_battlefield(&engine, p0, phyrexian_battleflies()).expect("the Battleflies landed");
    assert!(
        keywords(&engine, flies).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(pt(&engine, flies), (0, 1), "and the body it prints is 0/1");

    // The first of the two activations the card allows. Ability 0 is the only
    // one the card has, and it is no mana ability, so it uses the stack.
    activate(&mut engine, p0, phyrexian_battleflies(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, flies), (1, 1), "+1/+0 on the first activation");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "one {{B}} was spent to pay for it"
    );

    // The second, which is the last the card permits.
    activate(&mut engine, p0, phyrexian_battleflies(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, flies),
        (2, 1),
        "the two pumps add, so the second one landed on top of the first"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "two {{B}} spent and one still floating"
    );

    // The third, which the cap refuses. The pool is read in the same breath as
    // the offer, because a missing ability with mana behind it is the cap and
    // a missing ability without it is only the price.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == flies),
        "\"Activate no more than twice each turn\": the third is not offered \
         even though a {{B}} is floating to pay for it: {:?}",
        legal.abilities
    );

    // Across the table and back: "until end of turn" is a duration and "each
    // turn" is a window, and the one turn cycle is what tells them from a
    // permanent counter and a once-a-game ability.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, flies),
        (0, 1),
        "the two +1/+0 are gone: the pump expired with the turn that made it"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4,
        "the untap step gave all four Swamps back"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == flies),
        "\"each turn\" and not each game: on the next turn the ability is \
         offered again: {:?}",
        legal.abilities
    );
}
