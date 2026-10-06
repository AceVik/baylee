//! `cards/artifacts/mv_4/skull_catapult.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "eef931d8-4048-4c33-bd8c-0f67d1083ee6"

/// Skull Catapult prints one line — "{1}, {T}, Sacrifice a creature: This
/// artifact deals 2 damage to any target" — and its three prices land in three
/// different places: the {1} out of a pool only the Forests paid into, the {T}
/// as a status change on the artifact itself, and the creature in its owner's
/// graveyard rather than merely gone. "A creature" is where the menu does the
/// work, because the Elf across the table is as much a creature as mine and
/// must not be on it, while "any target" (CR 115.4) carries both seats in the
/// same choice — which is what lets the 2 damage be aimed at the 1/1 opposite
/// instead of at the player behind it.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
fn skull_catapult_eats_a_creature_of_its_own_side_for_two_damage_at_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[skull_catapult()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the fodder is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The {4} is read off the *pool*: five untapped Forests pay nothing until
    // they are tapped, and `can_afford` never looks at the lands themselves.
    let card = in_hand(&engine, p0, skull_catapult()).expect("the Catapult is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}, and `can_afford` reads the pool rather \
         than the untapped lands: {:?}",
        legal.castable
    );

    // Five Forests, and the Elf named as the printing kept back: it is the
    // creature the ability is about to ask for, and a mana creature tapped for
    // the cost would put its own {{G}} in the pool (#159).
    tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests and no Elf: five green"
    );
    cast_with_floating(&mut engine, p0, skull_catapult());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let catapult = on_battlefield(&engine, p0, skull_catapult()).expect("the Catapult resolved");
    assert!(!is_tapped(&engine, catapult), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{4}} is spent and exactly the {{1}} the ability charges is left"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(catapult, 0)),
        "with the {{1}} floating and a creature to eat, the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skull_catapult(), 0);

    // Both halves of the activation, answered in the order they arrive rather
    // than in the order they are expected: the target at CR 601.2c, the mana,
    // the tap and the sacrifice at CR 601.2h.
    let mut asked_whom = false;
    let mut menu: Option<Vec<ObjectId>> = None;
    for _ in 0..12 {
        if asked_whom && menu.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat aims it");
                assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
                assert!(
                    options.contains(&fodder) && options.contains(&theirs),
                    "\"any target\" is any creature, on either side of the table: {options:?}"
                );
                assert!(
                    !options.contains(&catapult),
                    "the Catapult is an artifact and no creature, and it is not a \
                     legal target for its own ability: {options:?}"
                );
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "CR 115.4: \"any target\" counts players in the same choice: \
                     {player_options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![theirs],
                            players: vec![],
                        },
                    )
                    .expect("their Elf was one of the options it enumerated");
                asked_whom = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the \
                     two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
                assert_eq!(
                    options,
                    vec![fodder],
                    "the one creature this seat controls is the whole menu: the \
                     Catapult is an artifact, the Forests are lands, and the Elf \
                     across the table is not yours to give up"
                );
                menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the creature the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Catapult's activation resolves: {other:?}"),
        }
    }
    assert!(asked_whom, "\"any target\" is a target choice");
    assert!(
        menu.is_some(),
        "the sacrifice is a cost and is asked before it is paid"
    );

    assert!(
        is_tapped(&engine, catapult),
        "{{T}} is part of the price, and CR 601.2h pays it with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} it charges came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the sacrifice itself"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing has happened to the creature it named yet: the damage is the \
         resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the player \
         whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, skull_catapult()).is_some(),
        "the Catapult outlives the creature it ate"
    );
}
