//! `cards/creatures/mv_2/selesnya_evangel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Selesnya Evangel prints “`{1}`, `{T}`, Tap an untapped creature you
/// control: Create a 1/1 green Saproling creature token.” The cost has
/// three parts and each is visible in a different place: the `{1}` in the
/// pool (`can_afford` reads the pool, so the line is missing entirely
/// without floating mana instead of being rejected), its own tap symbol on
/// the Evangel, and the second tap on an *other* creature that must belong
/// to you — the Elf across the table is the control that “you control” is
/// read and not just “a creature”. Only the one Forest is tapped for mana:
/// its own Elf stays untapped, because it is about to be the second tap,
/// and an Elf tapped for mana would no longer be an untapped creature.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn selesnya_evangel_taps_two_creatures_for_one_green_saproling() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), selesnya_evangel(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let evangel = on_battlefield(&engine, p0, selesnya_evangel()).expect("the Evangel is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(evangel, 0)),
        "without {{1}} in the pool the price is unpayable and the line is \
         missing from the offer: {:?}",
        legal.abilities
    );

    // Only the Forest: the Elf is the creature that should tap the second
    // part of the price in a moment.
    let taken = tap_mana_where(&mut engine, p0, |id| id == land);
    assert_eq!(taken, 1, "a Forest and nothing else");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "ein grüner, und grün zahlt die {{1}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(evangel, 0)),
        "with {{1}} in the pool the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, selesnya_evangel(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the price asks for the second creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "der aktivierende Sitz antwortet seine eigene Kostenfrage"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "a cost and not a search: a tapped permanent is not destroyed"
    );
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&elves),
        "my own Elf is untapped and may pay too: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"a creature *you* control\": the Elf across the table is not mine: {options:?}"
    );
    assert!(
        !options.contains(&evangel),
        "the Evangel's own {{T}} is half the price, so it cannot also be the \
         creature it taps (CR 118.3): {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the creature that offered the question pays the price");

    assert!(
        is_tapped(&engine, evangel),
        "{{T}} ist der eigene Teil des Preises"
    );
    assert!(is_tapped(&engine, elves), "and the second tip hits the Elf");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} came from the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "creating a creature is not a mana ability, so it is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "eine Aktivierung, ein Saproling");
    let saproling = tokens[0];
    let kinds = types(&engine, saproling);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "a Saproling is a creature: {kinds:?}"
    );
    let printed = engine
        .state()
        .object(saproling)
        .expect("the Saproling is on the battlefield")
        .token
        .expect("es weiß, welches Token es ist");
    assert_eq!(printed.name, "Saproling");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "ein 1/1, wie gedruckt"
    );
    assert!(
        printed.colors.contains(baylee_core::color::Color::Green),
        "und ein *grünes* Saproling"
    );
    assert!(
        on_battlefield(&engine, p0, selesnya_evangel()).is_some(),
        "the Evangel remains on the battlefield — the ability sacrifices nothing"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf on the table has never done anything"
    );
}
