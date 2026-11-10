//! `cards/instants/mv_4/lightning_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lightning Blast — {3}{R} instant: "Lightning Blast deals 4 damage to any
/// target."
///
/// Two casts off one pool of eight Mountains read the whole card inside a
/// single main phase. The first is aimed at the opponent, so the life they are
/// missing afterwards is the printed number *exactly* — sixteen, not the
/// seventeen a three-damage spell would leave nor the fifteen a five-damage
/// one — and the menu it was chosen from carries a creature and both players
/// in one choice, which is what CR 115.4's "any target" means. The second cast
/// is aimed at that creature and the same four damage kills it, so the object
/// half of the target spec is played and not merely offered; the pool read in
/// between says a {3}{R} really costs four of the eight.
#[test]
#[allow(clippy::too_many_lines)]
fn lightning_blast_deals_four_to_a_player_and_to_a_creature_off_the_mana_it_charges() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 8])
        .hand(0, &[lightning_blast(), lightning_blast()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is across the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1 for four damage to kill"
    );

    // Eight Mountains are the whole board, and whether a `{3}{R}` is castable
    // is read off the pool rather than off the untapped lands — so the mana
    // goes in before anything is cast.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Mountains tapped for eight red"
    );

    // The first cast, at the player. `any target` is a question with two lists,
    // and both of them are read before it is answered.
    cast_with_floating(&mut engine, p0, lightning_blast());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "\"any target\" reaches a creature: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4 counts players in the same choice: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target for `any target`");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "\"deals 4 damage\": sixteen, and not the seventeen a three-damage \
         spell would leave nor the fifteen a five-damage one"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage went to the seat that was named and not to the caster"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the first {{3}}{{R}} came out of the eight, and the pool did not \
         empty between the two casts (CR 500.5)"
    );

    // The second cast, at the creature: the other half of `AnyTarget`, read on
    // a body instead of on a life total.
    cast_with_floating(&mut engine, p0, lightning_blast());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "the creature across the table is on the menu: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "four damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        mine(&engine, p0, lightning_blast(), Zone::Graveyard).len(),
        2,
        "both spells resolved: one copy in the graveyard would mean one of them \
         never left the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "eight Mountains paid two {{3}}{{R}} and not one mana is left over"
    );
}
