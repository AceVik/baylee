//! `cards/creatures/mv_1/thornscape_apprentice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thornscape Apprentice prints two activations and no other text: "{R}, {T}:
/// Target creature gains first strike until end of turn." and "{W}, {T}: Tap
/// target creature." Both are paid for with the tap symbol, so one card can
/// show only one of them before it is spent — hence two Apprentices, one per
/// colour, played in one main phase off a Mountain and a Plains. The two
/// Goblins are the other half of each claim: "target creature" is read off the
/// offer (the opponent's Goblin is on it) and the effect lands on the creature
/// that was named and on no other.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn thornscape_apprentice_trades_its_tap_for_red_first_strike_and_a_white_tap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(87, forest())
        .battlefield(
            0,
            &[
                mountain(),
                plains(),
                thornscape_apprentice(),
                thornscape_apprentice(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let apprentices = all_on_battlefield(&engine, p0, thornscape_apprentice());
    assert_eq!(apprentices.len(), 2, "two Apprentices, one per colour");
    let (red, white) = (apprentices[0], apprentices[1]);
    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin is out");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a printed 1/1 before anything is asked"
    );

    // Mana first: `legal.abilities` is filtered through `can_afford`, which
    // reads the pool, so an offer read off an empty pool would say nothing.
    // A Mountain, a Plains and nothing else — neither Goblin taps for mana, so
    // those two are the whole of the pool.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "the Mountain");
    assert_eq!(pool.available(ManaColor::White), 1, "and the Plains");
    assert_eq!(pool.total(), 2, "and nothing else on this board makes mana");

    // {R}, {T}: target creature gains first strike.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, index)| *src == red && *index == 0)
        .expect("the {R} ability is offered once red mana floats");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    // CR 601.2c picks the target and CR 601.2h pays afterwards, so both halves
    // of the price are read here, behind the answer rather than before it.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "the {{R}} is paid after the target is named"
    );
    assert!(
        is_tapped(&engine, red),
        "and {{T}} is paid in the same step"
    );
    assert!(
        !stack_is_empty(&engine),
        "a keyword grant is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "the creature that was named has first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and the creature it did not name has none"
    );
    assert!(
        !keywords(&engine, red).contains(KeywordSet::FIRST_STRIKE),
        "the Apprentice grants the keyword, it does not keep it"
    );

    // {W}, {T}: tap target creature — the second Apprentice, because the first
    // one is spent and its ability costs the tap symbol.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, index)| *src == white && *index == 1)
        .expect("the {W} ability is offered once white mana floats");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the tap targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature here too: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, theirs),
        "the ability taps the creature it was aimed at, across the table"
    );
    assert!(
        !is_tapped(&engine, mine),
        "and leaves the creature it did not name standing"
    );
    assert!(
        is_tapped(&engine, white),
        "{{T}} paid the other half of the cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{W}} beside it — two Apprentices, two colours, nothing left"
    );
}
