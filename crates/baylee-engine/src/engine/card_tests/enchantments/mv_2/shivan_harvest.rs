//! `cards/enchantments/mv_2/shivan_harvest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shivan Harvest — {1}{R} enchantment: "{1}{R}, Sacrifice a creature:
/// Destroy target nonbasic land."
///
/// Both halves of the price are read somewhere a test can see them — the {1}{R}
/// leaves a pool four Mountains actually filled, and the creature leaves the
/// battlefield for its owner's graveyard — while the ability is on the stack,
/// which is what tells a real sacrifice into a real activation. The words that
/// need a witness on the board are the two filters: the Elf across the table
/// must stay off the sacrifice menu (CR 701.21a) and the Forest beside the
/// targeted land must stay off the target menu, so it is read twice — once in
/// the options and once by that Forest still standing at the end.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every clause of the card read off it
fn shivan_harvest_sacrifices_a_creature_to_destroy_a_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[shivan_harvest()])
        // A nonbasic land this seat does not control, and a basic one beside
        // it in the same seat's hands.
        .battlefield(1, &[treetop_village(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The enchantment arrives first, off **two** named Mountains and not off
    // whatever is standing: the other two are what the ability spends
    // further down, and the pool emptying in between is what makes each
    // price a number rather than a leftover. `cast_from_hand` would have
    // tapped the Elf as well, and the Elf is the creature this ability eats.
    let mountains = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(mountains.len(), 4, "four Mountains were dealt");
    let (first, second) = (mountains[0], mountains[1]);
    tap_mana_where(&mut engine, p0, |id| id == first || id == second);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, which is the {{1}}{{R}} the enchantment costs"
    );
    cast_with_floating(&mut engine, p0, shivan_harvest());
    pass_until(&mut engine, stack_is_empty);
    let harvest = on_battlefield(&engine, p0, shivan_harvest()).expect("the Harvest resolved");
    let victim = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let nonbasic = on_battlefield(&engine, p1, treetop_village()).expect("their Treetop Village");
    let basic_land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent the pool: nothing is floating for the ability"
    );

    // `legal.abilities` is behind `can_afford`, which reads the pool and not
    // the untapped lands, so the {1}{R} is floating *before* anything is
    // claimed about the offer.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Mountains nobody spent on the enchantment; the Elf is kept \
         back because it is the creature about to be eaten"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(harvest, 0)),
        "with {{1}}{{R}} floating the Harvest's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, shivan_harvest(), 0);

    // **The target first and the cost second**, which is the order CR 601.2
    // gives and CR 602.2b applies to an activation: the land is named at
    // CR 601.2c and the creature is eaten at CR 601.2h. A test that asked
    // for the sacrifice first would have been asserting the opposite order
    // and passing on a board where it does not matter.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target nonbasic land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&nonbasic),
        "a nonbasic land across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&basic_land),
        "\"nonbasic\" is read and not skipped: the Forest beside it is no target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![nonbasic],
            },
        )
        .expect("the land the question offered was chosen");

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
            "the sacrifice is a cost and is asked after the target: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options,
        vec![victim],
        "the creature you control is the whole of the answer"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice: {options:?}"
    );
    assert!(
        !options.contains(&harvest),
        "the Harvest is an enchantment: it cannot eat itself: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the ability is on the stack"
    );

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "`CR 601.2h`: the sacrifice is paid before the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} that was floating went with it"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, treetop_village()).is_none(),
        "the targeted land left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, treetop_village()).is_some(),
        "and it is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the basic land the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, shivan_harvest()).is_some(),
        "the enchantment paid its price and stayed on the table"
    );
}
