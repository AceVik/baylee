//! `cards/creatures/mv_3/wirewood_savage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wirewood Savage — {2}{G}, 2/2 Elf: "Whenever a Beast enters, you may draw
/// a card."
///
/// Both readings of the "may" come off one Savage, because a clause nobody
/// ever declines is a clause nobody has read: a Beast arrives and the draw is
/// refused, which has to leave the library exactly as long as the cast found
/// it, and a second Beast arrives and the draw is taken, which has to move one
/// card off the top of the library and into the hand. Between them walks a
/// Llanowar Elves — a creature, and no Beast — so the trigger's subtype filter
/// is read and not assumed: `pass_until` answers every "you may" it meets, so
/// a trigger on the Elf would show up as a drawn card in the counts below.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wirewood_savage_draws_for_a_beast_only_when_the_may_is_taken() {
    let p0 = PlayerId::new(0);
    let mut field = basics();
    field.insert(0, wirewood_savage());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &field)
        .hand(
            0,
            &[shaleskin_bruiser(), llanowar_elves(), shaleskin_bruiser()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert!(
        on_battlefield(&engine, p0, wirewood_savage()).is_some(),
        "the Savage is the trigger's source, and it starts on the table"
    );

    // Mana first: `castable` is read off the pool and not off the untapped
    // lands, and `basics()` is four of every basic, so no colour below decides
    // whether a Beast can arrive at all.
    let lands = lands_of(&engine, p0).len();
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, lands,
        "every basic is a route whose whole price is its own {{T}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "and `basics()` is four of every basic, so the pool holds the four \
         green the Elf needs and the mana both Beasts need"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // The first Beast: the question is asked, and declined.
    cast_with_floating(&mut engine, p0, shaleskin_bruiser());
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
        unreachable!("pass_until stops on nothing but the question")
    };
    assert_eq!(player, p0, "the Savage's controller is the one asked");
    assert!(
        on_battlefield(&engine, p0, shaleskin_bruiser()).is_some(),
        "the Beast is on the battlefield before its own entry is answered"
    );
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is one of the two answers the question offers");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "\"you may draw a card\" — declined, so nothing left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the Beast left the hand and nothing replaced it"
    );

    // A creature, and no Beast. The trigger reads the subtype, so this cast
    // must ask nothing at all.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf resolved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "an Elf is no Beast: entering asked nothing and drew nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2,
        "one Beast and one Elf have left the hand, and nothing has entered it"
    );

    // The second Beast: the same question, answered the other way.
    cast_with_floating(&mut engine, p0, shaleskin_bruiser());
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
        unreachable!("pass_until stops on nothing but the question")
    };
    assert_eq!(player, p0, "and the second entry asks the same seat");
    let library_at_question = library_size(&engine, p0);
    let hand_at_question = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("taking the draw is the other answer the question offers");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_at_question - 1,
        "\"you may draw a card\" — taken, so one card left the top of the \
         library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_at_question + 1,
        "and it is in hand, which is the half a library count alone cannot see"
    );
}
