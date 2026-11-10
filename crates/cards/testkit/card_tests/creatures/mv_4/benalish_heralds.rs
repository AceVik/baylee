//! `cards/creatures/mv_4/benalish_heralds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Benalish Heralds — {3}{W} Human Soldier 2/4 — "{3}{U}, {T}: Draw a card."
///
/// Both halves of the price are read in one activation and each is legible in
/// a different place: the {3}{U} out of a pool the Islands actually filled,
/// the tap symbol in the creature's own status, and the draw as a library one
/// shorter with a hand one longer — the half no pool reading can see. The
/// Herald itself is cast on turn one, so the printed body and its {3}{W}
/// arrive for real; the ability is only pressed on its controller's next
/// turn, because CR 302.6 leaves a creature that arrived this turn no {T} to
/// pay with at all. Exactly the four Islands are tapped and the Herald is
/// named as the thing kept out of it, so "the pool it empties is the pool the
/// ability filled" is a claim about the ability and not about a board.
#[test]
fn benalish_heralds_pays_four_mana_and_its_own_tap_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[benalish_heralds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The body arrives the way the card arrives: {3}{W} off an open board.
    cast_from_hand(&mut engine, p0, benalish_heralds());
    pass_until(&mut engine, stack_is_empty);
    let herald = on_battlefield(&engine, p0, benalish_heralds()).expect("the Herald resolved");
    assert_eq!(pt(&engine, herald), (2, 4), "the body the card prints");

    // And the ability waits for its own turn: a creature that entered this
    // turn cannot pay a {T} cost (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, herald),
        "the untap step stood it back up"
    );

    // Four Islands and nothing else: the printed price is {3}{U} *and* the tap
    // symbol, so the pool the ability spends has to be a pool that was filled
    // for it, and the Herald is named as the thing kept out of that.
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands tapped, four blue floating"
    );
    assert!(
        !is_tapped(&engine, herald),
        "and nothing has tapped the Herald yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(herald, 0)),
        "with {{3}}{{U}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, benalish_heralds(), 0);
    assert!(
        is_tapped(&engine, herald),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so it uses the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}} came out of the pool the four Islands filled"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand — an emptied library would satisfy the count above \
         without drawing anything"
    );
    assert!(
        on_battlefield(&engine, p0, benalish_heralds()).is_some(),
        "the ability costs the creature nothing but its tap, so the 2/4 is \
         still on the table and still tapped"
    );
}
