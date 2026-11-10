//! `cards/creatures/mv_2/wirewood_hivemaster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wirewood Hivemaster — {1}{G} Elf 1/1: "Whenever another nontoken Elf
/// enters, you may create a 1/1 green Insect creature token."
///
/// Two Llanowar Elves are cast in the same main phase, so one board reads all
/// three words of the trigger at once: the Hivemaster's *own* entry makes no
/// token (`another`), the first Elf's entry asks a `MayDo` question that is
/// accepted and leaves exactly one 1/1 green Insect, and the second Elf's entry
/// asks again and is declined so the count stays at one — a trigger that never
/// asked, or a "may" that resolved without its answer, would move that number
/// in one direction or the other. All four Forests are tapped before anything
/// is cast and the Elves are cast off mana already floating, so the second Elf
/// is not paid for by the first one, which by then is a creature on the table.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wirewood_hivemaster_asks_per_elf_and_makes_a_green_insect_for_each_yes() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(
            0,
            &[wirewood_hivemaster(), llanowar_elves(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, and the Elves still in hand are not mana sources yet"
    );

    // {1}{G} off the pool, and the Hivemaster is an Elf entering under its own
    // trigger — "another" is the word under test, so no token may appear.
    cast_with_floating(&mut engine, p0, wirewood_hivemaster());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, wirewood_hivemaster()).is_some(),
        "the Hivemaster resolved onto the battlefield"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "\"another nontoken Elf\" — the Hivemaster is itself an Elf and its \
         own entry does not trigger it"
    );

    // The first Elf: the trigger asks, and the offer is taken.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the may-question the trigger asked");
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one nontoken Elf entered and the offer was accepted"
    );
    let insect = engine
        .state()
        .object(tokens[0])
        .expect("the Insect is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(
        (insect.power, insect.toughness),
        (Some(1), Some(1)),
        "a 1/1 Insect"
    );
    assert!(
        insect.colors.contains(baylee_core::color::Color::Green),
        "and a *green* one"
    );

    // The second Elf: the same trigger, answered the other way.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("a second question, and it takes the other answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "\"you may\" — declining leaves the board exactly where it was, so a \
         token-per-Elf reading would show two here"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} and the two {{G}} came out of the four Forests"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))
            })
            .count(),
        2,
        "both Elves resolved and are standing beside the Hivemaster"
    );
}
