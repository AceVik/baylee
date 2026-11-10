//! `cards/lands/utility/goblin_burrows.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ad65bb8e-57de-49f3-ba7c-be62cf3fe3df"

/// Goblin Burrows is a colourless land printing two lines: "{T}: Add {C}" and
/// "{1}{R}, {T}: Target Goblin creature gets +2/+0 until end of turn."
///
/// The filter is the whole second line, so the board carries a Goblin on each
/// side of the table and a non-Goblin creature beside them: the card says
/// "target Goblin creature" and never "you control", so the opponent's Goblin
/// is a legal target while the Baleful Strix is not — and only the creature
/// that was named may grow. Both halves of the price are read where the rules
/// put them: the target is named first (CR 601.2c), so the land is still
/// untapped and the two red are still floating while the question stands, and
/// the `{T}` and the `{1}{R}` go with the answer (CR 601.2h).
///
/// The lands are tapped through `tap_all_mana_but` naming the Burrows itself,
/// because its own `{T}: Add {C}` has its own tap as its whole price (#159) and
/// `tap_all_mana` would have spent the very activation under test.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn goblin_burrows_pumps_the_goblin_it_names_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                goblin_burrows(),
                mountain(),
                mountain(),
                festering_goblin(),
                baleful_strix(),
            ],
        )
        // The Goblin the filter must offer without being asked whose it is.
        .battlefield(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let burrows = on_battlefield(&engine, p0, goblin_burrows()).expect("the Burrows are out");
    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin is out");
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("the Strix is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(pt(&engine, theirs), (1, 1), "and so is the one across it");

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands, so both halves of the offer are read before
    // anything is tapped: the mana line costs its own tap and nothing else,
    // and the pump wants `{1}{R}` that is nowhere yet.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Burrows holds the main phase");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats yet"
    );
    assert!(
        legal.abilities.contains(&(burrows, 0)),
        "the printed `{{T}}`: Add `{{C}}` is offered on an empty pool: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(burrows, 1)),
        "`{{1}}{{R}}, {{T}}` is not payable with nothing in the pool, and an \
         unaffordable cost is absent from the offer rather than refused: {:?}",
        legal.abilities
    );

    // The Burrows is named as the printing kept back: its whole mana ability is
    // its own `{T}` (#159), so the helper would have spent the land this test
    // then activates by hand. The two Mountains fill the pool instead.
    tap_all_mana_but(&mut engine, p0, Some(goblin_burrows()));
    assert!(
        !is_tapped(&engine, burrows),
        "the source kept back is still standing"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        2,
        "two Mountains tapped, and nothing off the Burrows"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        2,
        "and both are red, which is what the printed {{R}} needs"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(burrows, 1)),
        "with {{1}}{{R}} floating the pump is offered — ability 1, behind the \
         land's own mana line: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_burrows(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Goblin creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&mine),
        "a Goblin this seat controls is on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target Goblin creature\" names no side of the table, so the Goblin \
         across it is offered too: {options:?}"
    );
    assert!(
        !options.contains(&strix),
        "the Baleful Strix is a creature and no Goblin: {options:?}"
    );
    assert!(
        !options.contains(&burrows),
        "the Burrows is a land and no creature: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, burrows),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}}{{R}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        is_tapped(&engine, burrows),
        "{{T}} is half the price and is paid by the land itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing has grown yet: the pump is the resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+2/+0 on the Goblin the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the other Goblin was offered and not named, so it is untouched"
    );
    assert_eq!(
        pt(&engine, strix),
        (1, 1),
        "and a creature that is no Goblin is not a target at all"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_burrows()).is_some(),
        "the price was a tap and two mana, so the land is still standing"
    );
}
