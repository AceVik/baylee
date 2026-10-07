//! `cards/creatures/mv_4/skyshroud_poacher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyshroud Poacher — {2}{G}{G}, a 2/2 — "{3}, {T}: Search your library for
/// an Elf permanent card, put it onto the battlefield, then shuffle."
///
/// `Find::BATTLEFIELD` is the word worth playing, because the three zones the
/// found card could land in each read differently: the library shrinks by one,
/// the hand does not grow, and the very card the search offered is a permanent
/// on the battlefield under the seat that searched. The deck is Llanowar Elves,
/// so the filter has a real menu to accept rather than an empty one. The
/// Poacher starts on the battlefield, because its {T} can be paid only by a
/// creature held since the turn began (CR 302.6), and three Forests are
/// exactly the {3} its ability charges — the pool then reads zero, so the
/// price was paid and not assumed.
#[test]
#[allow(clippy::too_many_lines)]
fn skyshroud_poacher_taps_and_three_mana_to_put_an_elf_card_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest(), skyshroud_poacher()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Forests into the pool: exactly the {3} the ability charges.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped, three green"
    );

    let poacher = on_battlefield(&engine, p0, skyshroud_poacher()).expect("the Poacher is out");
    assert_eq!(pt(&engine, poacher), (2, 2), "the body the card prints");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool rather than the untapped lands — so the claim is made with the
    // mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(poacher, 0)),
        "with {{3}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, skyshroud_poacher(), 0);
    assert!(
        is_tapped(&engine, poacher),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "searching is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that activated does the searching");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        options.len(),
        library_before,
        "every card in the library is an Elf permanent the filter accepts"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("the found card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\" — the very card the search offered is a \
         permanent now"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and it is an Elf permanent under the control of the seat that searched"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the card went to the battlefield and not to hand, so the hand is the \
         size it was"
    );
    assert!(
        on_battlefield(&engine, p0, skyshroud_poacher()).is_some(),
        "the price was a tap and three mana, so the Poacher stays to find another"
    );
}
