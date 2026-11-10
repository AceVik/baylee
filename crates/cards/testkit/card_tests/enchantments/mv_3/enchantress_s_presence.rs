//! `cards/enchantments/mv_3/enchantress_s_presence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Enchantress's Presence — {2}{G} enchantment: "Whenever you cast an
/// enchantment spell, draw a card."
///
/// The trigger reads the *type of the spell on the stack*, so the hand holds
/// one of each kind: the Presence itself, which cannot draw for its own cast
/// (CR 603.2 — it is not on the battlefield yet when its own spell is cast),
/// a Llanowar Elves, which is a spell and no enchantment, and Fastbond, which
/// is the one card here that satisfies the filter. The library is read after
/// each of the three, so "a creature spell is no enchantment spell" is a
/// claim and not an assumption — a trigger that had lost its type filter
/// would draw on the Elves and every count before it would still have read
/// correctly.
#[test]
fn enchantresss_presence_draws_for_an_enchantment_and_for_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[enchantress_s_presence(), llanowar_elves(), fastbond()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Forests are the whole curve — {2}{G}, then {G}, then {G} — and
    // nothing else on this board makes mana, so the pool read after each cast
    // is a statement about the lands and nothing else.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped, and the Elves are still in hand"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The negative nobody has to control: the trigger lives on a permanent,
    // and that permanent arrives only once its own spell has resolved.
    cast_with_floating(&mut engine, p0, enchantress_s_presence());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, enchantress_s_presence()).is_some(),
        "the Presence resolved onto the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "\"whenever you *cast*\" — the Presence was not on the battlefield \
         when its own cast went by, so it drew nothing for it"
    );

    // A creature spell is the filter's other half: the same cast event with
    // the wrong type on it.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves resolved past the Presence's trigger, not through it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "a creature spell is no enchantment spell, so the Presence looked at \
         it and drew nothing"
    );

    // And the card the trigger is written about. The trigger goes on the stack
    // above the spell that caused it (CR 603.3b), so the card is drawn before
    // Fastbond itself ever resolves.
    cast_with_floating(&mut engine, p0, fastbond());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, fastbond()).is_some(),
        "Fastbond resolved: the enchantment landed behind its own trigger"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — exactly one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2,
        "three cards cast and one drawn, so the hand is two smaller — a draw \
         that emptied the library without filling the hand would satisfy the \
         count above"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the five Forests paid exactly {{2}}{{G}} + {{G}} + {{G}}"
    );
}
