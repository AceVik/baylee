//! `cards/sorceries/mv_4/unyaro_bee_sting.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a500313b-35e3-4ebc-9144-e9486784757b"

/// Unyaro Bee Sting — {3}{G} sorcery: "Unyaro Bee Sting deals 2 damage to any
/// target."
///
/// "Any target" (CR 115.4) is the half no reading of the card file can settle,
/// so the menu the spell publishes is read before it is answered: one Elf under
/// the caster and one across the table are both object options, and both seats
/// are player options in that same choice. The damage then lands where it was
/// aimed — two on a printed 1/1 is lethal (CR 704.5g), so the Elf across the
/// table dies while the one beside the caster stays a 1/1 — and the opponent's
/// life total is the control that says the spell hit the creature and not the
/// player whose board it stood on. The {3}{G} is a real payment: the four
/// Forests fill the pool and the pool is empty once the price is in.
#[test]
fn unyaro_bee_sting_deals_two_damage_to_the_any_target_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[unyaro_bee_sting()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "a printed 1/1 for two damage to kill"
    );

    // Four Forests into the pool, with the Elf named as the printing kept
    // back: a mana creature counts in the pool like any other source, so
    // tapping it here would make "the {3}{G} was paid" a claim about five
    // mana instead of about the lands.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, four green, and the Elf contributed nothing"
    );

    cast_with_floating(&mut engine, p0, unyaro_bee_sting());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"any target\" reaches creatures on either side of the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: and players are part of that same choice: {player_options:?}"
    );

    // The target is named first (CR 601.2c) and the cost is paid last
    // (CR 601.2h), so the mana is still floating while the question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{G}} came out of the pool when the spell was cast"
    );
    assert!(
        on_stack(&engine, unyaro_bee_sting()).is_some(),
        "and the spell is on the stack with nothing left to decide"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage kills a printed 1/1 (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it left the battlefield rather than merely being marked"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the creature the spell did not name is untouched"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the \
         player whose board it stood on"
    );
    assert_eq!(engine.state().players[0].life, 20, "nor to the caster");
    assert!(
        in_graveyard(&engine, p0, unyaro_bee_sting()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}
