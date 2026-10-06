//! `cards/creatures/mv_3/seton_krosan_protector.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seton, Krosan Protector prints one line: "Tap an untapped Druid you
/// control: Add {G}." The whole price is the tap of a permanent the filter
/// names, so the board carries one of each answer and the menu has to tell
/// them apart: a Druid under this seat (Llanowar Elves), a creature that is
/// no Druid (Festering Goblin), and a Druid across the table, which is not
/// this seat's to tap. Nothing on the board is tapped before the mana is
/// read, so the single green in the pool can only be the ability's own — no
/// land and no ability of the tapped Elf's was pressed to get it.
#[test]
fn seton_krosan_protector_taps_a_druid_you_control_for_one_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                seton_krosan_protector(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let seton = on_battlefield(&engine, p0, seton_krosan_protector()).expect("Seton is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("the Goblin is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the ability is pressed"
    );
    assert!(
        !is_tapped(&engine, elf),
        "and the Druid the price is about is untapped"
    );

    activate(&mut engine, p0, seton_krosan_protector(), 0);
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
            "the cost asks which Druid is tapped, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "a tap being paid for, and not a search"
    );
    assert_eq!((min, max), (1, 1), "one Druid, no more and no fewer");
    assert!(
        options.contains(&elf),
        "the untapped Druid you control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&goblin),
        "a Goblin is no Druid: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a seat taps only what it controls, so the Druid across the table is \
         not on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Druid the question offered pays the cost");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"Add {{G}}\" — one green, and the empty pool before it is what says \
         the ability is where it came from"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&engine, elf),
        "the Druid that paid the cost is tapped"
    );
    assert!(
        !is_tapped(&engine, seton),
        "the price was a Druid and not the source: Seton is still standing"
    );
    assert!(
        !is_tapped(&engine, goblin),
        "the permanent the filter declined never moved"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and neither did the Druid across the table"
    );
}
