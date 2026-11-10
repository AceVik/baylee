//! `cards/artifacts/mv_4/rod_of_ruin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rod of Ruin prints one line — "{3}, {T}: This artifact deals 1 damage to any
/// target" — and both halves of it are the engine's answer rather than the
/// card's, so both are played on one board. The {3} is a real payment: seven
/// Forests fill the pool, the {4} takes four of it, and the offer is only read
/// once the remaining three are already floating, because `can_afford` reads
/// the pool and not the untapped lands. The target is the other half: "any
/// target" is one selection over objects *and* players (CR 115.4), so the seat
/// is named and the Elf across the table — whose own 1/1 body makes the loss of
/// a life the only place the damage can show — must still be standing.
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
#[test]
fn rod_of_ruin_taps_and_three_mana_to_shoot_the_target_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 7])
        .hand(0, &[rod_of_ruin()])
        // A creature across the table, so the question has an object to offer
        // beside the seats and the damage has somewhere to *not* go.
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // {4} out of the seven Forests, which leaves exactly the {3} the ability
    // charges floating beside it: the whole scenario stays inside this one main
    // phase, and CR 500.5 empties a pool only when a step ends.
    cast_from_hand(&mut engine, p0, rod_of_ruin());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, rod_of_ruin()).is_some()
    });
    let rod = on_battlefield(&engine, p0, rod_of_ruin()).expect("the Rod resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 to aim past");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{4}} is spent and the {{3}} the ability charges is still floating"
    );
    assert!(!is_tapped(&engine, rod), "an artifact enters untapped");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool — which is why the claim is made with the mana already there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(rod, 0)),
        "the one line the card prints, now that its {{3}} is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, rod_of_ruin(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        !options.contains(&rod),
        "the Rod is an artifact and no creature, so it cannot be its own target: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, rod),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} is still in the pool for the same reason"
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

    assert!(is_tapped(&engine, rod), "{{T}} is paid by the artifact");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage\" to the seat that was named — one, and never a point per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the ability did not name never moved, so the one point \
         went to the target and not to the board"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and it still carries the body it was printed with"
    );
    assert!(
        on_battlefield(&engine, p0, rod_of_ruin()).is_some(),
        "an activated ability costs the artifact nothing but its tap"
    );
}
