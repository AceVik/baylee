//! `cards/lands/utility/mirkwood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mirkwood enters tapped, taps for {B} or {G}, and then spends the whole
/// card: "{2}{B}{G}, {T}, Sacrifice this land: Put two +1/+1 counters on
/// target Bear, Spider, or Wolf you control. Activate only as a sorcery."
/// The scenario is the menu and the price in one play. The menu is one card
/// wide — the Young Wolf is the only legal target, while the Llanowar Elves
/// beside it is a creature you control of the wrong subtype and the Wolf
/// across the table is the right subtype under the wrong controller — so a
/// filter that had lost either `ControlledByYou` or the subtype list would
/// read the same on the card file and show up here. The board is honest about
/// the entry because the land is actually *played*: it arrives tapped, a real
/// untap step stands it back up, its own {T} is kept out of `tap_all_mana` so
/// the price can be paid at all, and it is gone to the graveyard the moment
/// the counters land.
#[test]
fn mirkwood_trades_itself_for_two_counters_on_the_wolf_you_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(907, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                swamp(),
                swamp(),
                young_wolf(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[young_wolf()])
        .hand(0, &[mirkwood()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A real land drop, so the printed entry applies: Mirkwood arrives
    // tapped, and nothing but an untap step may stand it back up.
    let land = play_land(&mut engine, p0, mirkwood());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "back at p0's own main");
    assert!(
        !is_tapped(&engine, land),
        "the untap step came round and stood it up"
    );

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, young_wolf()).expect("their Wolf is out");
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "a printed 1/1 before the counters"
    );

    // Mirkwood is the one source kept back: its own {T} is half of the price
    // the ability below pays, and `tap_all_mana` would have spent it.
    tap_all_mana_but(&mut engine, p0, Some(mirkwood()));
    let pool_before = engine.state().players[0].mana_pool.total();
    assert!(
        pool_before >= 4,
        "{{2}}{{B}}{{G}} is floating before the ability is claimed affordable"
    );

    // Ability 0 is the printed mana ability; the counter line is ability 1.
    activate(&mut engine, p0, mirkwood(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Bear, Spider, or Wolf you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        options,
        vec![wolf],
        "one Wolf you control — not the Elf (the wrong subtype) and not the \
         Wolf across the table (the wrong controller)"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .expect("the only option on the menu is the answer");

    // The price was paid last (CR 601.2h): the land is tapped and gone, and
    // what is left is the ability resolving.
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, wolf, CounterKind::P1P1),
        2,
        "\"Put two +1/+1 counters on target Bear, Spider, or Wolf you control\""
    );
    assert_eq!(pt(&engine, wolf), (3, 3), "and they are +1/+1 counters");
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the Elf beside it was never the target"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor was the Wolf across the table"
    );
    assert!(
        on_battlefield(&engine, p0, mirkwood()).is_none(),
        "the land sacrificed itself to pay for its own ability"
    );
    assert!(
        in_graveyard(&engine, p0, mirkwood()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 4,
        "the {{2}}{{B}}{{G}} came out of the pool"
    );
}
