//! `cards/creatures/mv_2/master_decoy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Master Decoy is a `{1}{W}` 1/2 whose whole card is one line: "`{W}`,
/// `{T}`: Tap target creature." The two halves of that price leave their
/// marks in two different places — the `{W}` in the mana pool and the `{T}`
/// on the creature itself — so the line has to be absent over an empty pool,
/// offered the moment white is floating, and paid for exactly when the
/// target is answered (CR 601.2c before CR 601.2h). Two Elves stand across
/// the table and only the one that was named may change state, which is what
/// tells "target creature" from "creatures"; the Decoy itself stays on the
/// menu as the other side of the same filter and as the permanent the tap
/// symbol spends.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn master_decoy_taps_the_creature_it_names_for_a_white_and_its_own_tap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(6174, forest())
        .battlefield(0, &[plains(), master_decoy()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let decoy = on_battlefield(&engine, p0, master_decoy()).expect("the Decoy is on the table");
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves across the table");
    let (victim, bystander) = (elves[0], elves[1]);
    assert_eq!(pt(&engine, decoy), (1, 2), "the printed 1/2 body");
    assert!(!is_tapped(&engine, decoy), "and it starts untapped");
    assert!(
        !is_tapped(&engine, victim) && !is_tapped(&engine, bystander),
        "as do both creatures it may aim at"
    );

    // `can_afford` reads the pool and not the untapped land, so with nothing
    // floating the line is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(decoy, 0)),
        "an empty pool pays no {{W}}, so the ability is absent rather than \
         refused: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Plains, and the Decoy prints no mana ability of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(decoy, 0)),
        "with {{W}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, master_decoy(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"tap target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&victim) && options.contains(&bystander),
        "\"target creature\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&decoy),
        "and the filter names no controller, so the Decoy is a legal target \
         for its own ability: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "the two Elves and the Decoy, and the Plains is no creature: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while this question stands, neither half of
    // the price has been paid.
    assert!(
        !is_tapped(&engine, decoy),
        "the {{T}} has not been spent yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{W}} is still in the pool"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    assert!(
        is_tapped(&engine, decoy),
        "{{T}} is the source's half of the price, paid as the activation ends"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{W}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, victim),
        "tapping is an effect and not a cost, so nothing has happened yet"
    );
    assert!(
        !stack_is_empty(&engine),
        "and tapping a creature is no mana ability, so the ability waits on \
         the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, victim),
        "\"tap target creature\" — the one the ability was aimed at"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "and only that one: the Elf nobody named never moved"
    );
    assert!(
        on_battlefield(&engine, p0, master_decoy()).is_some(),
        "the Decoy paid its own tap and is otherwise untouched"
    );
}
