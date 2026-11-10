//! `cards/creatures/mv_5/azami_lady_of_scrolls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "0f8b97fe-3e5e-47c2-9a9d-7f77482aa159"

/// Azami, Lady of Scrolls is a 0/2 legendary Human Wizard whose whole text is
/// "Tap an untapped Wizard you control: Draw a card", so nothing about the card
/// is readable off the printing: the subtype, the "you control", the untapped
/// half of the price and the draw are all the engine's answers. The board
/// therefore carries three untapped Wizards under Azami's control, an untapped
/// non-Wizard creature beside them and one Wizard across the table — the cost
/// question offers exactly the first three and never the table's Wizard.
/// Because the filter prints no "another", Azami pays with her own tap like the
/// others, and once all three are down the price has no candidate left and the
/// line stops being offered rather than being announced and refused.
#[test]
#[allow(clippy::too_many_lines)]
fn azami_taps_wizards_of_her_own_side_for_cards_until_none_are_left_untapped() {
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
                vendilion_clique(),
                tishanas_tidebinder(),
                llanowar_elves(),
            ],
        )
        // A Wizard across the table: "you control" is a word the menu has to
        // read, and an untapped same-kind permanent is the only thing that can
        // say so.
        .battlefield(1, &[vendilion_clique()])
        .hand(0, &[azami_lady_of_scrolls()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Islands pay {2}{U}{U}{U}, and the Elf is left standing on purpose:
    // an untapped creature of this seat is what shows the printed subtype is
    // read at all.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands and nothing else tapped"
    );
    cast_with_floating(&mut engine, p0, azami_lady_of_scrolls());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let azami = on_battlefield(&engine, p0, azami_lady_of_scrolls()).expect("Azami resolved");
    let clique = on_battlefield(&engine, p0, vendilion_clique()).expect("my Clique is out");
    let tidebinder =
        on_battlefield(&engine, p0, tishanas_tidebinder()).expect("the Tidebinder is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, vendilion_clique()).expect("their Clique is out");
    assert_eq!(pt(&engine, azami), (0, 2), "the body the card prints");
    assert!(!is_tapped(&engine, azami), "and she arrives untapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(azami, 0)),
        "the one line the card prints, offered over an empty pool because its \
         whole price is a tap: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // (1) The cost question, which is where the card is: three untapped Wizards
    // this seat controls — Azami among them, because the filter prints no
    // "another" — and neither the Wizard across the table nor the untapped Elf
    // beside her.
    activate(&mut engine, p0, azami_lady_of_scrolls(), 0);
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
            "tapping a Wizard is a cost, so the engine asks which one: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostTap,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one Wizard, and the cost asks once");
    assert_eq!(
        options.len(),
        3,
        "the three untapped Wizards this seat controls: {options:?}"
    );
    for wizard in [azami, clique, tidebinder] {
        assert!(
            options.contains(&wizard),
            "a Wizard this seat controls is a legal price: {options:?}"
        );
    }
    assert!(
        !options.contains(&elf),
        "the Elf is an untapped creature this seat controls and no Wizard: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\": a Wizard across the table is not yours to tap: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![clique],
            },
        )
        .expect("the Wizard the question offered pays the cost");
    assert!(
        is_tapped(&engine, clique),
        "tapping that Wizard is the whole price, and CR 118.3 wants one that \
         was untapped"
    );
    assert!(
        !is_tapped(&engine, azami) && !is_tapped(&engine, tidebinder),
        "and nothing else was tapped with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the \
         count above"
    );

    // (2) The Clique is tapped now, so the menu is one smaller: "an untapped
    // Wizard" is a reading of its own and a tapped one is no price.
    activate(&mut engine, p0, azami_lady_of_scrolls(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the cost question again, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 2, "the Wizards still standing: {options:?}");
    assert!(
        !options.contains(&clique),
        "a tapped Wizard may not pay the same tap twice: {options:?}"
    );
    assert!(
        options.contains(&tidebinder) && options.contains(&azami),
        "while the other two are still offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tidebinder],
            },
        )
        .expect("the Wizard the question offered pays the cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    // (3) Azami herself is the last untapped Wizard this seat has, which is the
    // half of the filter with no "another" in it.
    activate(&mut engine, p0, azami_lady_of_scrolls(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the cost question once more, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![azami],
        "the only untapped Wizard left is the Lady of Scrolls herself"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![azami],
            },
        )
        .expect("Azami may pay her own price");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        is_tapped(&engine, azami),
        "and paying it tapped her like any other Wizard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "three activations, three cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 3,
        "and all three reached the hand"
    );

    // (4) Nothing untapped is left to pay with, so the line is gone from the
    // offer rather than announced and refused.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(azami, 0)),
        "\"tap an untapped Wizard you control\" wants an untapped Wizard: with \
         all three of this seat's Wizards down there is no price to pay and \
         nothing is offered — and the opponent's Clique standing untapped is \
         not this seat's to tap: {:?}",
        legal.abilities
    );
    assert!(
        on_battlefield(&engine, p0, azami_lady_of_scrolls()).is_some(),
        "the cost was the tap and not the permanent: she is still on the battlefield"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the Wizard across the table never moved"
    );
}
