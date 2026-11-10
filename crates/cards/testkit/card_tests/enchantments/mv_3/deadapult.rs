//! `cards/enchantments/mv_3/deadapult.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deadapult — {2}{R} Enchantment: "{R}, Sacrifice a Zombie: This
/// enchantment deals 2 damage to any target."
///
/// Both words of the cost need a witness on the board: the sacrifice names a
/// *Zombie* and nothing else, so my Festering Goblin (a Zombie Goblin) is the
/// whole menu while the Wurm beside it and the Zombie across the table stay
/// off it — a filter that had dropped `HasSubtype(ZOMBIE)` or
/// `ControlledByYou` would still read correctly on the card file. The {R} is
/// paid last (CR 601.2h), out of a pool four Mountains filled for a {2}{R}
/// cast that leaves exactly one red behind, and "any target" (CR 115.4) is the
/// object-and-player choice that is read by aiming the two damage at the
/// *player*, whose life total then says two. The sacrificed Goblin's own death
/// trigger is aimed at the Wurm, a 6/6 that survives the −1/−1 and whose 5/5
/// afterwards is what says that trigger really resolved.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn deadapult_eats_a_zombie_for_two_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                festering_goblin(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[deadapult()])
        // A Zombie across the table: "a Zombie" is not an invitation to eat
        // somebody else's.
        .battlefield(1, &[festering_goblin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Four Mountains and nothing else that taps for mana: neither creature
    // prints a mana ability, so the pool is exactly the four the lands made.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and no other source on this board"
    );
    cast_with_floating(&mut engine, p0, deadapult());
    pass_until(&mut engine, stack_is_empty);
    let enchantment = on_battlefield(&engine, p0, deadapult()).expect("Deadapult resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} is spent and the {{R}} the ability charges is still \
         floating — one main phase, so CR 500.5 never emptied the pool"
    );

    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Zombie is out");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("a non-Zombie is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Zombie is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "with the {{R}} already floating, the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, deadapult(), 0);

    // CR 601.2c first: the target is named while the Zombie still stands and
    // the {{R}} is still in the pool, because the costs come last.
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
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice, either \
         seat of the table: {player_options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&wurm) && options.contains(&theirs),
        "and the creatures on both sides of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2h pays last, so nothing has been spent while this stands"
    );
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "and the Zombie is still standing to be sacrificed"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the seat across the table was one of the options");

    // CR 601.2h: the sacrifice, and with it the {R}, are the last thing paid.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Zombie, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one Zombie, no more and no fewer");
    assert_eq!(
        options,
        vec![mine],
        "the one creature with the Zombie subtype under your control is the \
         whole of the answer"
    );
    assert!(
        !options.contains(&wurm),
        "the Wurm is a creature and no Zombie: `HasSubtype(ZOMBIE)` is read, \
         not skipped"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: a Zombie across the table is not yours to sacrifice"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Zombie the question offered pays the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{R}} went with it"
    );
    assert!(
        in_graveyard(&engine, p0, festering_goblin()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    // Festering Goblin's own printed trigger ("when this dies, target
    // creature gets −1/−1") fires off that sacrifice. It is not the card
    // under test, so it is aimed at the Wurm.
    let mut aimed_at_the_wurm = false;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert!(
                    options.contains(&wurm) && options.contains(&theirs),
                    "the death trigger wants a creature, on either side of the \
                     table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![wurm],
                        },
                    )
                    .expect("the Wurm was one of the options");
                aimed_at_the_wurm = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected after the sacrifice: {other:?}"),
        }
    }
    assert!(
        aimed_at_the_wurm,
        "the sacrificed Zombie's own death trigger is a question before \
         anything resolves"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"deals 2 damage to any target\" — two, aimed at the player and not \
         at a creature"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the life belongs to the seat that was aimed at"
    );
    assert!(
        on_battlefield(&engine, p0, deadapult()).is_some(),
        "the enchantment does not sacrifice itself to pay its own cost"
    );
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the Zombie the ability did not name never moved"
    );
    assert_eq!(
        pt(&engine, wurm),
        (5, 5),
        "the death trigger resolved on the Wurm: −1/−1 on a printed 6/6"
    );
}
