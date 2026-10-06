//! `cards/creatures/mv_3/verduran_enchantress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Verduran Enchantress — `{1}{G}{G}`, a 0/2 — prints one line: "Whenever
/// you cast an enchantment spell, you may draw a card."
///
/// Three spells are cast off one tapping of six Forests, and each of them is
/// a different half of that sentence. The first Fastbond is an enchantment of
/// the Enchantress' own controller, so the trigger asks and the accepted draw
/// costs exactly one card off the top of the library — the library and not the
/// hand, because the spell leaving the hand and the card arriving cancel out
/// there. The second Fastbond is answered "no", which is the whole of the word
/// "may". And the Llanowar Elves is a spell of the same seat that is no
/// enchantment at all, so the type half of the trigger's filter is read
/// against a cast that really happened rather than assumed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn verduran_enchantress_draws_for_an_enchantment_of_its_controller_and_declines_the_rest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(
            0,
            &[
                verduran_enchantress(),
                fastbond(),
                fastbond(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Mana before the claim: `can_afford` reads the pool and never the
    // untapped lands, and the whole scenario plays inside this one main
    // phase, so CR 500.5 does not empty what the six Forests fill.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests, six green — and nothing else on the board makes mana"
    );

    cast_with_floating(&mut engine, p0, verduran_enchantress());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, verduran_enchantress()).is_some(),
        "the Enchantress resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and her {{1}}{{G}}{{G}} came out of the pool"
    );

    // The trigger's own enchantment, answered "yes". The library is read
    // before the cast and after the question, so the number that moves is the
    // card the ability offered and nothing else.
    let library = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, fastbond());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "\"whenever you cast\": the seat that cast the spell is the seat asked"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, fastbond()).is_some(),
        "the enchantment itself resolved once its trigger was through"
    );
    assert_eq!(
        library_size(&engine, p0),
        library - 1,
        "\"you may draw a card\", accepted: exactly one card off the top"
    );

    // The same cast again, answered the other way. "May" is the whole of what
    // separates the two readings, and a trigger that drew on a refusal would
    // have satisfied every count above it.
    let library = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, fastbond());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        all_on_battlefield(&engine, p0, fastbond()).len(),
        2,
        "two enchantments cast and two resolved, so the question really was asked twice"
    );
    assert_eq!(
        library_size(&engine, p0),
        library,
        "\"you may\" declined: the library is untouched"
    );

    // And a spell of the same seat that is no enchantment at all: the filter's
    // type half, read against a cast that really happened.
    let library = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature spell resolved, so the silence around it is not a cast \
         that never happened"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and its {{G}} came out of the pool the Forests filled"
    );
    assert_eq!(
        library_size(&engine, p0),
        library,
        "no question and no draw for a creature spell: the trigger reads \
         \"enchantment\""
    );
}
