//! `cards/creatures/mv_1/goblin_sledder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Sledder — {R}, 1/1 Goblin — "Sacrifice a Goblin: Target creature
/// gets +1/+1 until end of turn."
///
/// Neither printed half is legible from the card file: the cost names no
/// particular Goblin, so the engine has to ask which one (CR 701.21a, which
/// is also why the question is a cost and not targeting), and the target is
/// *any* creature. So the board gives every word a counter-example — the
/// Elf beside the Sledder is a creature its controller controls and is
/// still not on the menu, and the Sledder across the table is a Goblin and
/// is not either. The Sledder is on its own menu (the card prints no
/// "another"), so it pays for its own pump, and targets come before costs
/// (CR 601.2c, then 601.2h) — which the scenario reads off the board while
/// the sacrifice is still being asked for.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn goblin_sledder_eats_a_goblin_to_pump_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        // A Goblin that is not ours, so "you control" is read and not assumed.
        .battlefield(1, &[llanowar_elves(), goblin_sledder()])
        .hand(0, &[goblin_sledder()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, goblin_sledder());
    pass_until(&mut engine, stack_is_empty);
    let sledder = on_battlefield(&engine, p0, goblin_sledder()).expect("the Sledder resolved");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_sledder =
        on_battlefield(&engine, p1, goblin_sledder()).expect("their Sledder is out");
    let land = on_battlefield(&engine, p0, mountain()).expect("a Mountain is out");
    assert_eq!(pt(&engine, sledder), (1, 1), "a printed 1/1");
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "and so is the Elf it will pump"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(sledder, 0)),
        "a Goblin is on the board, so the one line the Sledder prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_sledder(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&sledder) && options.contains(&my_elf) && options.contains(&their_elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Mountain is no creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the creature the question offered is the creature the pump aims at");

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
            "the sacrifice is the question that follows the target: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that pays is the seat that chose");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one Goblin, and the cost asks once");
    assert_eq!(
        options.len(),
        1,
        "one Goblin under your control: {options:?}"
    );
    assert!(
        options.contains(&sledder),
        "the Sledder is a Goblin you control, so it is on its own menu — the \
         card prints no \"another\" and may pay for itself: {options:?}"
    );
    assert!(
        !options.contains(&my_elf),
        "a creature you control that is no Goblin is not yours to feed it: {options:?}"
    );
    assert!(
        !options.contains(&their_sledder),
        "and a Goblin an opponent controls is not yours to sacrifice \
         (CR 701.21a), however the filter reads: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Mountain is not a creature at all: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_sledder()).is_some(),
        "costs are paid last (CR 601.2h), so the Sledder is still standing \
         while the question is open"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![their_sledder],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sledder],
            },
        )
        .expect("the Goblin the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "a sacrifice is no mana ability: the pump goes on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, their_elf),
        (2, 2),
        "\"target creature gets +1/+1 until end of turn\""
    );
    assert_eq!(
        pt(&engine, my_elf),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_sledder()).is_none(),
        "the Sledder paid the cost with itself"
    );
    assert!(
        in_graveyard(&engine, p0, goblin_sledder()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, goblin_sledder()).is_some(),
        "and the Sledder across the table never moved"
    );
}
