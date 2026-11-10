//! `cards/sorceries/mv_5/essence_drain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Essence Drain — {4}{B} sorcery: "Essence Drain deals 3 damage to any target
/// and you gain 3 life."
///
/// Every word of that sentence is the engine's answer rather than the card's,
/// so one cast reads all of them. "Any target" (CR 115.4) is a single choice
/// spanning objects *and* players, so the menu is read with an Elf across the
/// table and both seats in it before the damage is aimed at the opponent —
/// where 20 becoming 17 says "3", and not one point per mana spent. The gain
/// is the other half and is read on the caster, and the empty pool afterwards
/// says the {4}{B} was really paid rather than the spell merely announced.
#[test]
fn essence_drain_deals_three_to_any_target_and_gains_three_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[essence_drain()])
        // A creature across the table, so the target question has an object to
        // offer beside the seats and the damage has somewhere to *not* go.
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the five untapped Swamps: with nothing floating the {4}{B}
    // is unpayable, so the sorcery is not among the castable cards at all.
    let card = in_hand(&engine, p0, essence_drain()).expect("the sorcery is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}{{B}}, and the offer reads the pool: {:?}",
        legal.castable
    );

    // Five Swamps into the pool, and the spell cast out of it while it is
    // floating — which is where the engine reads affordability from.
    cast_from_hand(&mut engine, p0, essence_drain());
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
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // five black are still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the cost is the last step of the cast, so nothing is spent yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "\"deals 3 damage\" to the seat that was named — three, and never a \
         point per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "\"and you gain 3 life\" belongs to the caster, not to the target"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{B}} came out of the pool the Swamps filled"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and it still carries the body it was printed with"
    );
    assert!(
        in_graveyard(&engine, p0, essence_drain()).is_some(),
        "the sorcery itself resolved and went to its owner's graveyard"
    );
}
