//! `cards/creatures/mv_3/quagmire_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Quagmire Druid prints one line — "{G}, {T}, Sacrifice a creature: Destroy
/// target enchantment" — and every part of it is the engine's answer rather
/// than the card's. An enchantment on either side of the table is what says
/// the target is any enchantment and not one this seat controls, while the one
/// that is not named stays behind afterwards. The two creatures are the ends of
/// CR 701.21a: the Elf beside the Druid pays, the Elf across the table is never
/// on the menu, and the Druid's own body is offered to itself because the card
/// says "a creature" and not "another". The single Forest is the whole of the
/// {G}, tapped before anything is claimed about the offer, because
/// `can_afford` reads the pool and not the untapped lands.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every gate it passes asserted
fn quagmire_druid_trades_a_creature_and_a_forest_for_an_enchantment_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[quagmire_druid(), forest(), llanowar_elves(), fastbond()],
        )
        .battlefield(1, &[llanowar_elves(), fastbond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let druid = on_battlefield(&engine, p0, quagmire_druid()).expect("the Druid stands");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("a creature to sacrifice");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their creature");
    let mine = on_battlefield(&engine, p0, fastbond()).expect("my enchantment");
    let target = on_battlefield(&engine, p1, fastbond()).expect("their enchantment");

    // The price that is not the {G} is the Druid's own {T} plus a creature, so
    // the Elf is named as the one source kept back: it is the creature the cost
    // is about, and a Forest tapped for the fixer is a pool of exactly one.
    let taken = tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        taken, 1,
        "the Forest, and nothing else on this board taps for mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one green floating for the {{G}}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(druid, 0)),
        "with the {{G}} in the pool the whole price is payable, so the one line \
         the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, quagmire_druid(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one enchantment, no more and no fewer");
    assert!(
        options.contains(&target) && options.contains(&mine),
        "\"target enchantment\" reaches either side of the table, so both are \
         offered: {options:?}"
    );
    assert!(
        !options.contains(&theirs) && !options.contains(&fodder) && !options.contains(&druid),
        "a creature is no enchantment: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );
    // CR 601.2c before CR 601.2h: the target is named while the {G} is still
    // floating and the Druid is still standing, so both halves of the price are
    // read after the answer.
    assert!(
        !is_tapped(&engine, druid) && engine.state().players[0].mana_pool.total() == 1,
        "nothing is paid while the target question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the enchantment the question offered was chosen");

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
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert!(
        options.contains(&fodder),
        "the Elf beside the Druid is a creature you control: {options:?}"
    );
    assert!(
        options.contains(&druid),
        "\"a creature\" is not \"another creature\": the Druid is on its own \
         menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's creature is not yours to sacrifice: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability is on the stack"
    );
    assert!(
        is_tapped(&engine, druid),
        "{{T}} was the other half of the price"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, fastbond()).is_some(),
        "\"destroy target enchantment\" — the card that was named is in its \
         owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, fastbond()).is_none(),
        "and off the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, fastbond()).is_some(),
        "the enchantment the ability never named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the opponent's creature was never fodder"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} came out of the pool"
    );
}
