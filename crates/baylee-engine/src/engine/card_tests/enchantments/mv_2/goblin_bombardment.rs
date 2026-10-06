//! `cards/enchantments/mv_2/goblin_bombardment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Bombardment ({1}{R}, enchantment) prints one line: "Sacrifice a
/// creature: This enchantment deals 1 damage to any target." Both halves of
/// the price and both halves of the target need their own witness, so the
/// board carries two Elves under the acting seat and one across the table:
/// the sacrifice menu must hold exactly the two that seat controls — never
/// the Elf it is aimed at, never the enchantment — while the target menu
/// must reach across the table (CR 115.4). Two activations play the object
/// and the player half of "any target" in turn, because the enchantment
/// survives both, and 20 → 19 is the only number that separates "1 damage"
/// from a damage count read off the board.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn goblin_bombardment_sacrifices_a_creature_to_deal_one_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[goblin_bombardment()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, goblin_bombardment());
    pass_until(&mut engine, stack_is_empty);
    let bomb = on_battlefield(&engine, p0, goblin_bombardment()).expect("the enchantment resolved");
    assert!(
        types(&engine, bomb).contains(TypeSet::ENCHANTMENT),
        "and what landed is the enchantment it prints, not a creature"
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two creatures to give up, one per activation"
    );
    let (first, second) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // The price is a creature and no mana at all, so the offer turns on
    // nothing the pool could have supplied — but read it after the cast, so
    // the enchantment is on the battlefield the offer is read from.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bomb, 0)),
        "a creature to sacrifice is the whole price, so the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    // First activation: the object half of "any target". CR 601.2c picks the
    // target and CR 601.2h pays afterwards, so nothing has been given up
    // while the question stands.
    activate(&mut engine, p0, goblin_bombardment(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that activated is the one that aims it"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both creatures this seat controls are legal targets: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"any target\" reaches across the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: the player half of the same choice names both seats: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");

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
            "the sacrifice is a cost and asks which one, got {:?}",
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
        options.len(),
        2,
        "the two creatures this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both Elves of mine are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: the creature the ability is aimed at is not mine to \
         sacrifice, so it is a target and never a price: {options:?}"
    );
    assert!(
        !options.contains(&bomb),
        "the enchantment is no creature: it cannot eat itself: {options:?}"
    );
    // CR 601.2h pays last: the target is named, the price is not yet paid,
    // and the creature that is about to die is still on the battlefield.
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nothing has been sacrificed while the cost question stands"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the target has not moved either"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the sacrificed creature went to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![second],
        "and only the creature that was named: the second Elf never moved"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was aimed at and not to the \
         player whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_bombardment()).is_some(),
        "the enchantment outlives the creature it ate — it is not sacrificed"
    );

    // Second activation: the player half of "any target", off the one
    // creature still standing.
    activate(&mut engine, p0, goblin_bombardment(), 0);
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "a player is a legal target: {player_options:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a face is the other half of `any target`");

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the price is asked again, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![second],
        "one creature left under this seat, so the menu is down to it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the last creature pays the price");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — exactly one, off the player half \
         of the same choice"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the seat that aimed it, not to the caster"
    );
    assert!(
        all_on_battlefield(&engine, p0, llanowar_elves()).is_empty(),
        "two activations, two creatures given up"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_bombardment()).is_some(),
        "and the enchantment is still there to do it a third time"
    );
}
