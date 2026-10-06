//! `cards/creatures/mv_4/moggcatcher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4845dc21-7534-4896-b298-4d44cd7a3b27"

/// Moggcatcher is a {2}{R}{R} 2/2 whose whole printed text is "{3}, {T}: Search
/// your library for a Goblin permanent card, put it onto the battlefield, then
/// shuffle." Both halves of that price are the engine's answer rather than the
/// card's, so one board reads all of them. The Moggcatcher is seated before the
/// game starts rather than cast, because a creature that arrived this turn may
/// not pay a {T} (CR 302.6); three Mountains beside it are exactly the {3} the
/// ability charges, the line is offered only once that mana is floating
/// (`can_afford` reads the pool and not the untapped lands), and the activation
/// takes the creature's own {T} with it. The library is built out of
/// Festering Goblins — a Goblin *permanent* card — while a single Mountain
/// moved into the same library is the control the printed filter needs: the
/// menu may name the Goblins and must decline the land. Where the card lands is
/// the last word, because "onto the battlefield" is what tells this from an
/// ordinary tutor, and the found card is read there rather than in a hand.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn moggcatcher_searches_a_goblin_permanent_onto_the_battlefield_for_three_and_its_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, festering_goblin())
        .battlefield(
            0,
            &[
                moggcatcher(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);

    // One of the four Mountains goes into the library: "a Goblin permanent
    // card" is the whole filter, and a land in the same library is the only
    // card on this board that can say whether the filter was read at all.
    let land = on_battlefield(&engine, p0, mountain()).expect("a Mountain is out");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            land,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");

    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let cat = on_battlefield(&engine, p0, moggcatcher()).expect("the Moggcatcher is seated");
    assert!(
        !is_tapped(&engine, cat),
        "it stands untapped, so its {{T}} is still there to pay"
    );

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the three untapped Mountains, so the line is not offered
    // before a single source has been tapped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cat, 0)),
        "an empty pool pays no {{3}}: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains are left on the battlefield, so exactly the {{3}} the \
         ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cat, 0)),
        "with {{3}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, moggcatcher(), 0);

    // The ability names no target, so CR 601.2h pays the whole price with the
    // announcement: the tap and the three mana are gone before anything can be
    // passed to.
    assert!(is_tapped(&engine, cat), "{{T}} is half the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a search is no mana ability, so the ability is waiting on the stack"
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
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library holds Goblin cards to find"
    );
    assert!(
        !options.contains(&land),
        "\"a Goblin permanent card\" — the Mountain the harness put in the same \
         library is a card and no Goblin: {options:?}"
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
            .expect("the found card is an object")
            .zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\" — a card in hand would be a tutor, and \
         this is not one"
    );
    assert_eq!(
        mine(&engine, p0, festering_goblin(), Zone::Battlefield).len(),
        1,
        "one activation, one Goblin on the battlefield, and not a copy per \
         library search"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );

    // The whole price was a tap and three mana, and the tap is spent, so the
    // line is no longer one the seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cat, 0)),
        "a tapped Moggcatcher has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
