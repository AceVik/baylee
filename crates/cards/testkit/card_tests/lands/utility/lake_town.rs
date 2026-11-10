//! `cards/lands/utility/lake_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lake-town prints four clauses: it enters tapped, its `{T}` adds {W} or
/// {U}, and `{2}{W}{U}, {T}, Sacrifice this land` puts two +1/+1 counters on
/// target Human you control, as a sorcery.
///
/// One main phase reads all of them. Two copies are only *placed* on the
/// battlefield by the harness — a placement is not an entry, so they stand
/// untapped — while the third is played for real, and that is what makes the
/// enters-tapped clause a claim about the land drop instead of about the
/// board the harness built. The target menu is then the card's own filter
/// read both ways, and the copy the activation sacrifices is the one that
/// paid its `{T}`, not the one the land drop left standing.
#[test]
#[allow(clippy::too_many_lines)] // four clauses, one board, one main phase
fn lake_town_enters_tapped_taps_for_white_or_blue_and_trades_itself_for_two_counters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                island(),
                lake_town(),
                lake_town(),
                auriok_bladewarden(),
                llanowar_elves(),
            ],
        )
        // A Human on the far side of the table: the card targets a Human
        // *you* control, and printing the same card for both seats keeps the
        // claim about the subtype to one reading.
        .battlefield(1, &[auriok_bladewarden()])
        .hand(0, &[lake_town()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let placed = all_on_battlefield(&engine, p0, lake_town());
    assert_eq!(
        placed.len(),
        2,
        "two copies were dealt straight onto the table"
    );
    assert!(
        placed.iter().all(|id| !is_tapped(&engine, *id)),
        "`starting_battlefield` places a permanent without an entry, so \
         neither of the two entered tapped"
    );

    let played = play_land(&mut engine, p0, lake_town());
    assert!(
        entered_tapped(&engine, played),
        "\"This land enters tapped\" — and the two placed copies are still \
         standing, so the tap is the land drop and not the board"
    );

    // {T}: Add {W} or {U}. Which of the two is a question (CR 605.1), and a
    // mana ability uses no stack (CR 605.3b): the answer is in the pool the
    // moment it is given.
    activate(&mut engine, p0, lake_town(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{W}} or {{U}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that tapped is the seat that names it");
    assert_eq!(
        options.len(),
        2,
        "the land prints two colors and offers no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "white and blue, the pair the card prints: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue is one of the answers it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the color that was named"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );

    // The four basics and the Elves pay the {2}{W}{U}; the one copy still
    // standing pays the {T} and is what the activation sacrifices. Mana
    // first: what the engine reads when it decides an ability is affordable
    // is the pool, not the board.
    tap_all_mana_but(&mut engine, p0, Some(lake_town()));
    let standing: Vec<ObjectId> = all_on_battlefield(&engine, p0, lake_town())
        .into_iter()
        .filter(|id| !is_tapped(&engine, *id))
        .collect();
    assert_eq!(
        standing.len(),
        1,
        "of three copies, only the one neither tapped nor just played can \
         still pay {{T}}: {standing:?}"
    );
    let paid = standing[0];

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(paid, 1)),
        "{{2}}{{W}}{{U}}, {{T}} and itself is payable on this board, so the \
         printed activation is offered: {:?}",
        legal.abilities
    );

    let human = on_battlefield(&engine, p0, auriok_bladewarden()).expect("my Human is out");
    let theirs = on_battlefield(&engine, p1, auriok_bladewarden()).expect("their Human is out");
    assert_eq!(
        counters_on(&engine, human, CounterKind::P1P1),
        0,
        "nothing has been put on it yet"
    );

    let floating = engine.state().players[0].mana_pool.total();
    activate(&mut engine, p0, lake_town(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the ability targets a Human, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat chooses its own target");
    assert_eq!(
        options,
        vec![human],
        "the Human this seat controls — not the Elves beside it and not the \
         Human across the table"
    );
    // Targets are chosen before costs are paid (CR 601.2c, then 601.2h), so
    // both halves of the price are still untouched while this question is
    // open.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating,
        "the mana is still in the pool"
    );
    assert!(
        mine(&engine, p0, lake_town(), Zone::Battlefield).contains(&paid),
        "and the land that will pay for it is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![human],
            },
        )
        .expect("the target the question offered");

    assert_eq!(
        in_graveyard(&engine, p0, lake_town()),
        Some(paid),
        "the cost is the last step: the copy that paid the {{T}} is the copy \
         that was sacrificed"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, human, CounterKind::P1P1),
        2,
        "\"Put two +1/+1 counters on target Human you control\""
    );
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::P1P1),
        0,
        "and nothing at all for the Human across the table"
    );
    assert!(
        mine(&engine, p0, lake_town(), Zone::Battlefield).contains(&played),
        "the land drop is still there: the ability ate the copy that paid its \
         {{T}} and not the one that was played"
    );
}
