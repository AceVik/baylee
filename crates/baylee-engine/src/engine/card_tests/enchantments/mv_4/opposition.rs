//! `cards/enchantments/mv_4/opposition.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Opposition — {2}{U}{U} enchantment: "Tap an untapped creature you control:
/// Tap target artifact, creature, or land."
///
/// Both halves of that sentence are the engine's answer rather than the card's,
/// so one activation has to be read off two menus and a board afterwards. The
/// price turns on the word *untapped*, which is why a second Elf is seated and
/// spent on the enchantment itself: it is down before the ability is ever
/// activated, so an offer that listed it would satisfy the same card file. The
/// target turns on the disjunction, so the artifact, the creature and the land
/// across the table are each asserted on the menu, and the two permanents
/// nobody named are read afterwards to show that "tap target" reaches exactly
/// one permanent.
#[test]
#[allow(clippy::too_many_lines)]
fn opposition_taps_an_untapped_creature_of_yours_to_tap_the_permanent_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[opposition()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which pays for the enchantment"
    );
    let (payer, spent) = (elves[0], elves[1]);
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Four Islands and one Elf pay the {2}{U}{U}, with `payer` named as the one
    // thing kept back: it is the creature the ability's price is about to ask
    // for, and a source tapped for mana is a source that is no longer untapped.
    tap_mana_except(&mut engine, p0, payer);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four Islands and the Elf beside them, which is every source on this board"
    );
    assert!(is_tapped(&engine, spent), "the Elf that paid is down");
    assert!(
        !is_tapped(&engine, payer),
        "and the one that was kept back is still standing"
    );

    cast_with_floating(&mut engine, p0, opposition());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let enchantment = on_battlefield(&engine, p0, opposition()).expect("the enchantment resolved");

    // The ability's whole price is a creature's tap and no mana at all, so the
    // offer is read with the pool still holding what the cast left behind —
    // which is the state `can_afford` reads.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, opposition(), 0);

    // One activation, two questions: the target at CR 601.2c and the creature
    // at CR 601.2h. Answered in the order they arrive rather than in the order
    // they are expected, and each menu is read on the spot.
    let mut cost_menu: Option<Vec<ObjectId>> = None;
    let mut target_menu: Option<Vec<ObjectId>> = None;
    for _ in 0..12 {
        if cost_menu.is_some() && target_menu.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
                if cost_menu.is_none() {
                    assert!(
                        !is_tapped(&engine, payer),
                        "CR 601.2h pays last: the creature that will pay the cost is \
                         still standing while the target is being chosen"
                    );
                }
                target_menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![their_rock],
                        },
                    )
                    .expect("the artifact across the table was one of the options");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat answers its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostTap,
                    "the variant is what tells a client this is a cost and not a search"
                );
                assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
                cost_menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![payer],
                        },
                    )
                    .expect("the untapped creature the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the ability resolves: {other:?}"),
        }
    }

    assert_eq!(
        cost_menu.expect("tapping a creature is a cost, so the engine asks which one"),
        vec![payer],
        "the whole menu is the one untapped creature this seat controls: the Elf \
         that paid for the enchantment is already down and the Elf across the \
         table is not yours to tap"
    );

    let menu = target_menu.expect("the ability targets, so the engine asks what");
    assert!(
        menu.contains(&their_rock),
        "\"target artifact\" reaches the Sol Ring across the table: {menu:?}"
    );
    assert!(
        menu.contains(&their_elf),
        "\"target creature\" reaches the Elf across the table: {menu:?}"
    );
    assert!(
        menu.contains(&their_forest),
        "and \"target land\" is the third type the disjunction names: {menu:?}"
    );
    assert!(
        menu.contains(&payer),
        "the filter names no controller, so a creature you control is one too: {menu:?}"
    );
    assert!(
        !menu.contains(&enchantment),
        "the enchantment is an artifact, a creature and a land none: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, their_rock),
        "the permanent the ability named is the one it tapped"
    );
    assert!(
        is_tapped(&engine, payer),
        "and tapping a creature you control was the price, paid by the Elf"
    );
    assert!(
        on_battlefield(&engine, p0, opposition()).is_some(),
        "an activated ability costs the enchantment nothing"
    );
    assert!(
        !is_tapped(&engine, their_elf) && !is_tapped(&engine, their_forest),
        "the two permanents nobody named are untouched, so the effect reaches the \
         target and not the board"
    );
}
