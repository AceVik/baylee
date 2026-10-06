//! `cards/creatures/mv_4/yawgmoth_thran_physician.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Yawgmoth, Thran Physician — {2}{B}{B} — 2/4 Legendary Creature — Human
/// Cleric, and the written half of a `Coverage::Partial` card: one life and
/// another creature buy a -1/-1 counter on up to one target creature and a
/// card.
///
/// A single activation is read four ways at once — the activating seat's life
/// total drops by exactly one, the creature named in the sacrifice menu leaves
/// the battlefield for its owner's graveyard while the Cleric stays, the
/// counter lands on the creature the ability was aimed at (a printed 2/2 that
/// survives it as a 1/1), and the draw takes one card off the top of the
/// library. Everything else on the board is the control for those four: the
/// Elf across the table is not this seat's to sacrifice and keeps no counter,
/// and the Cleric is the ability's own source and so cannot be the creature it
/// eats. The printed "up to one" is none or one, which the question's
/// `(min, max)` records. Naming none is played in `activation_target_tests`.
#[allow(clippy::too_many_lines)] // a three-part cost, the counter it places and the card it draws
#[test]
fn yawgmoth_pays_a_life_and_another_creature_for_a_minus_counter_and_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                skyclave_apparition(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[yawgmoth_thran_physician()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    cast_from_hand(&mut engine, p0, yawgmoth_thran_physician());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let yawgmoth =
        on_battlefield(&engine, p0, yawgmoth_thran_physician()).expect("the Cleric resolved");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves stand");
    let victim = on_battlefield(&engine, p0, skyclave_apparition()).expect("the Apparition stands");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");
    assert_eq!(pt(&engine, victim), (2, 2), "a printed 2/2 to aim at");
    assert_eq!(engine.state().players[0].life, 20, "and no life paid yet");
    let library_before = library_size(&engine, p0);

    // Ability 0 is the protection static; the one printed line that is
    // activated is index 1, and it costs no mana at all.
    activate(&mut engine, p0, yawgmoth_thran_physician(), 1);

    // One activation, two questions: which creature is targeted (CR 601.2c)
    // and which creature pays the cost (CR 601.2h). Answered in whichever
    // order they arrive.
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut aimed = false;
    for _ in 0..12 {
        if aimed && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat aims its own ability");
                assert_eq!(
                    (min, max),
                    (0, 1),
                    "\"up to one\" may name none, and this activation names one"
                );
                assert!(
                    options.contains(&victim),
                    "\"target creature\" offers the Apparition: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                assert!(
                    options.contains(&fodder) && options.contains(&victim),
                    "the two other creatures you control are the menu: {options:?}"
                );
                assert!(
                    !options.contains(&yawgmoth),
                    "\"another creature\" — the Cleric is its own source and no cost of its own: {options:?}"
                );
                assert!(
                    !options.contains(&theirs),
                    "CR 701.21a: an opponent's creature is not yours to sacrifice: {options:?}"
                );
                menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Yawgmoth's ability resolves: {other:?}"),
        }
    }
    assert!(aimed, "the ability targets, so a target was asked for");
    assert!(
        !menu.is_empty(),
        "and it sacrifices, so the cost was asked for"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\": exactly one, off the seat that activated the ability"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing off the other seat"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the creature named as the cost left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, yawgmoth_thran_physician()).is_some(),
        "the Cleric did not pay with itself"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf across the table never moved"
    );

    assert_eq!(
        counters_on(&engine, victim, baylee_cards_dsl::CounterKind::M1M1),
        1,
        "\"put a -1/-1 counter on ... target creature\""
    );
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "the projection reads it: a 2/2 with a -1/-1 counter is a 1/1"
    );
    assert_eq!(
        counters_on(&engine, yawgmoth, baylee_cards_dsl::CounterKind::M1M1),
        0,
        "the ability's own source was not the creature it counted"
    );
    assert_eq!(
        counters_on(&engine, theirs, baylee_cards_dsl::CounterKind::M1M1),
        0,
        "and the counter crossed no table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"and draw a card\": one off the top of the library"
    );
}
