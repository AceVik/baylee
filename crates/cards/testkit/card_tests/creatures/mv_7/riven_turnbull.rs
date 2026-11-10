//! `cards/creatures/mv_7/riven_turnbull.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Riven Turnbull is a {5}{U}{B} legendary 5/7 whose entire rules text is
/// "{T}: Add {B}", and nothing but playing it can tell the three claims apart:
/// that the printed cost is really seven mana, that the body is 5/7, and that
/// the tap symbol is the *whole* price of one black mana. The board is exactly
/// five Islands and two Swamps, so the pool reads seven and then zero — the
/// cost paid rather than merely announced — and the mana ability is read on a
/// later turn because a creature that arrived this turn has not been under its
/// controller's control since their turn began (CR 302.6). Tapping it then
/// leaves one black and no blue, with the stack empty (CR 605.3b) and every
/// land still standing, so nothing else on this board could have made it.
#[test]
#[allow(clippy::too_many_lines)]
fn riven_turnbull_costs_seven_mana_and_taps_itself_for_one_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[riven_turnbull()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `can_afford` reads the mana pool and not the untapped lands, so seven
    // lands that have made nothing yet are seven mana the engine has not seen.
    let card = in_hand(&engine, p0, riven_turnbull()).expect("the Advisor is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{5}}{{U}}{{B}}: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "five Islands and two Swamps, which is exactly the printed cost"
    );
    cast_with_floating(&mut engine, p0, riven_turnbull());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let turnbull = on_battlefield(&engine, p0, riven_turnbull()).expect("the Advisor resolved");
    assert_eq!(pt(&engine, turnbull), (5, 7), "the body the card prints");
    assert!(
        engine
            .state()
            .object(turnbull)
            .expect("the Advisor is an object")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "a legendary Advisor"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "seven mana went out of the pool, so the {{5}}{{U}}{{B}} was paid and \
         not merely printed"
    );

    // A whole turn cycle: the untap step stands the lands back up, and only
    // then is the creature's own {T} the price of anything (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, turnbull),
        "the untap step stood the Advisor back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(turnbull, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, riven_turnbull(), 0);
    assert!(
        is_tapped(&engine, turnbull),
        "{{T}} is the whole price and is paid with the activation"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{T}}: Add {{B}}");
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "black alone, though the card is blue and black"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    for land in lands_of(&engine, p0) {
        assert!(
            !is_tapped(&engine, land),
            "the lands never moved, so the black mana has no other source on \
             this board"
        );
    }

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(turnbull, 0)),
        "a tapped Advisor has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
