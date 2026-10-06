//! `cards/creatures/mv_1/orcish_lumberjack.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Orcish Lumberjack — {R}, a 1/1 Orc: "{T}, Sacrifice a Forest: Add three
/// mana in any combination of {R} and/or {G}."
///
/// The price is the whole card, and the menu the cost question enumerates is
/// where its words get read: "a Forest" is a land *type* — the Mountain
/// beside it is a land and no Forest — and CR 701.21a supplies the "you
/// control" that keeps the Forest across the table off it. The payment is
/// then named one point at a time, which is the "any combination" clause: the
/// pool starts empty, so every drop of it came off the Lumberjack, and two
/// red beside one green is an answer an engine that added three of one colour
/// cannot produce.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn orcish_lumberjack_sacrifices_a_forest_of_yours_for_three_mana_in_any_combination() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_lumberjack(), forest(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lumberjack =
        on_battlefield(&engine, p0, orcish_lumberjack()).expect("the Lumberjack is out");
    let mine = on_battlefield(&engine, p0, forest()).expect("a Forest under p0");
    let mountain = on_battlefield(&engine, p0, mountain()).expect("a Mountain under p0");
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing was tapped before the price, so every mana below came off the Lumberjack"
    );

    activate(&mut engine, p0, orcish_lumberjack(), 0);

    // CR 601.2h: the sacrifice is paid last, out of a menu the engine
    // publishes, and it is not a target.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Forest, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one Forest, no more and no fewer");
    assert!(
        options.contains(&mine),
        "the Forest you control is the whole of the answer: {options:?}"
    );
    assert_eq!(options.len(), 1, "and it is the whole menu: {options:?}");
    assert!(
        !options.contains(&mountain),
        "\"a Forest\" is a land type: a Mountain is a land and no Forest: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's Forest is not yours to sacrifice: {options:?}"
    );
    assert!(
        !options.contains(&lumberjack),
        "the Lumberjack is an Orc and no Forest: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Forest the question offered pays the cost");

    // "Add three mana in any combination of {R} and/or {G}": one point at a
    // time, each offering both colours. The three answers are read back out
    // of the pool, so a default colour would fail here rather than pass.
    let mut picks = [ManaColor::Red, ManaColor::Green, ManaColor::Red].into_iter();
    let mut asked = 0;
    while let Pending::ChooseColor { player, options } = engine.pending().clone() {
        assert_eq!(player, p0, "the activating seat names the colours");
        assert!(
            options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
            "\"any combination of {{R}} and/or {{G}}\" offers both: {options:?}"
        );
        let pick = picks
            .next()
            .expect("three mana is the whole of what the card prints");
        engine.apply(p0, PlayerAction::ChooseColor(pick)).unwrap();
        asked += 1;
    }
    assert_eq!(asked, 3, "one question per point of mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        2,
        "the two red that were named"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the one green — three of one colour would read 3 and 0 here"
    );
    assert_eq!(pool.total(), 3, "three mana and nothing else beside them");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, lumberjack), "{{T}} was half the price");
    assert!(
        !is_tapped(&engine, mountain),
        "the Mountain never moved, so the three mana has no other source on this board"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "and the sacrificed Forest is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "while the Forest across the table never moved"
    );
}
