//! `cards/creatures/mv_1/skirk_prospector.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skirk Prospector is a 1/1 Goblin whose whole printed text is one mana
/// ability: "Sacrifice a Goblin: Add {R}." The cost names a Goblin and not
/// *another* Goblin, so the Prospector stands on its own menu — the half a
/// filter carrying a stray `Filter::Another` would lose while every assertion
/// about the Goblin beside it went on passing. The Elf on this side and the
/// Prospector across the table read the other half, `Filter::ControlledByYou`:
/// a creature that is no Goblin, and a Goblin that is not yours. And "Add {R}"
/// is a mana ability (CR 605.1), so nothing reaches the stack (CR 605.3b) and
/// the red is in the pool the moment the answer lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn skirk_prospector_eats_a_goblin_you_control_for_one_red_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), skirk_prospector(), llanowar_elves()])
        .hand(0, &[skirk_prospector()])
        .battlefield(1, &[skirk_prospector()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {R} off the one Mountain, so the pool is empty again the moment the
    // second Prospector is paid for and whatever is in it afterwards came off
    // the ability under test.
    let hill = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    tap_mana_where(&mut engine, p0, |id| id == hill);
    cast_with_floating(&mut engine, p0, skirk_prospector());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{R}} was the whole cost of the second Prospector"
    );

    let gunners = all_on_battlefield(&engine, p0, skirk_prospector());
    assert_eq!(gunners.len(), 2, "one was dealt and one was cast");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, skirk_prospector()).expect("their Prospector is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal
            .abilities
            .iter()
            .any(|(source, index)| *index == 0 && gunners.contains(source)),
        "there is a Goblin to eat, so the one line the Prospector prints is \
         offered — and its price is a sacrifice and no mana, so an empty pool \
         withholds nothing: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skirk_prospector(), 0);
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
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one Goblin, no more and no fewer");
    for goblin in &gunners {
        assert!(
            options.contains(goblin),
            "both of this seat's Goblins are on the menu, the source among \
             them: \"sacrifice a Goblin\" is not \"sacrifice another\": {options:?}"
        );
    }
    assert!(
        !options.contains(&elf),
        "the Elves are a creature and no Goblin: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a seat sacrifices only what it controls, whatever the filter says: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    assert!(
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] },)
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gunners[0]],
            },
        )
        .expect("a Goblin the question offered pays the cost");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "`Add {{R}}` is in the pool the moment the answer lands"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "`CR 605.3b`: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        in_graveyard(&engine, p0, skirk_prospector()).is_some(),
        "the sacrificed Goblin is in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, skirk_prospector()).len(),
        1,
        "one Goblin was eaten and the other still stands"
    );
    assert!(
        on_battlefield(&engine, p1, skirk_prospector()).is_some(),
        "the opponent's Prospector never moved"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and neither did the creature that is no Goblin"
    );
}
