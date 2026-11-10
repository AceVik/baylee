//! `cards/instants/mv_1/spark_spray.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spark Spray is called that, but it is two cards: "{R} — Instant: Spark
/// Spray deals 1 damage to any target" and in hand "Cycling {R} ({R},
/// Discard this card: Draw a card.)". The battlefield plays both halves from
/// a single tapping of two Mountains, because a pool only empties at the end
/// of the step (CR 500.5): the second copy is cycled first and must discard
/// exactly itself to draw a card, and afterwards the remaining copy kills a
/// printed 1/1 opponent — while the second Elf remains and p1's life stays
/// at 20, which proves the targeting question "any target" (CR 115.4) on
/// both lists instead of assuming it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn spark_spray_cycles_itself_for_a_card_and_burns_a_creature_for_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[spark_spray(), spark_spray()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "zwei 1/1er, einer davon der Zuschauer");
    let (doomed, bystander) = (elves[0], elves[1]);

    // Both halves of the card cost {R}, so the two Mountains pay
    // once for the cycle and once for the spell.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red mana, and the creatures above them are tapped"
    );

    // The hand half: the cycling ability is offered on the card in hand,
    // not on a permanent.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == spark_spray()))
        })
        .expect("Cycling is offered for the card in hand");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the offered ability is payable");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        mine(&engine, p0, spark_spray(), Zone::Graveyard).len(),
        1,
        "\"Discard this card\" is the cost, and it takes the card that prints it"
    );
    assert!(
        in_hand(&engine, p0, spark_spray()).is_some(),
        "the other copy is still in hand: Cycling doesn't eat the card"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — a card from the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discard and the turn cancel each other out"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the first {{R}} is paid, the second is still floating for the spell"
    );

    // The spell half: 1 damage to a target that may be a creature.
    cast_with_floating(&mut engine, p0, spark_spray());
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
        unreachable!("pass_until stops only on a target question")
    };
    assert_eq!(player, p0, "der wirkende Sitz zielt");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&doomed) && options.contains(&bystander),
        "\"any target\" reaches either creature: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: a player is an \"any target\" too: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("der Elf war eine der angebotenen Optionen");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "1 damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and only the named creature: the other Elf is still standing"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature and not to the player behind it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{R}} was the spell's price"
    );
    assert_eq!(
        mine(&engine, p0, spark_spray(), Zone::Graveyard).len(),
        2,
        "both halves of the card are played: once canceled, once cast"
    );
}
