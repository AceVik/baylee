//! `cards/artifacts/mv_3/phyrexian_altar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Altar — {3} artifact: "Sacrifice a creature: Add one mana of any
/// color." It is a *mana* ability whose whole price is a creature, so playing
/// it asks the two questions no reading of the card file answers: which
/// permanent is being given up and which of the five colors the mana is. The
/// Elf is the offering worth naming — the Altar is an artifact and the Forests
/// are lands, so a menu that had lost `Filter::YOUR_CREATURE` would still offer
/// something and still pass — and the Elf across the table is the CR 701.21a
/// half: a seat sacrifices only what it controls. The price is a creature and
/// no mana at all, so the pool is emptied before the activation, which is what
/// makes "one black and nothing else" afterwards an exact statement.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn phyrexian_altar_sacrifices_a_creature_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[phyrexian_altar()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests into the pool, and the Elf named as the source kept back:
    // it is the creature the Altar is about to eat, and `tap_all_mana` would
    // have drunk its own `{T}: Add {G}` as well (#159).
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests in the pool, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, phyrexian_altar());
    pass_until(&mut engine, stack_is_empty);
    let altar = on_battlefield(&engine, p0, phyrexian_altar()).expect("the Altar resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} is spent: the whole price of what follows is a creature"
    );
    assert!(!is_tapped(&engine, altar), "and the Altar is untapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(altar, 0)),
        "with a creature on the board the one line the Altar prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, phyrexian_altar(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert_eq!(
        options,
        vec![fodder],
        "the creature you control is the whole of the answer: the Altar is an \
         artifact, the Forests are lands, and the Elf across the table is not \
         yours to give up"
    );
    assert!(
        !options.contains(&altar),
        "the Altar is an artifact: it cannot eat itself"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that paid the price names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one creature, and nothing else: the board floats no \
         green, so the count above is exact"
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
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and it is in its owner's graveyard, which is where a sacrificed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_altar()).is_some(),
        "the Altar outlives the creature it ate"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's Elves never moved"
    );
}
