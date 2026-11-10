//! `cards/creatures/mv_3/barbarian_lunatic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barbarian Lunatic prints one sentence — "{2}{R}, Sacrifice this creature:
/// It deals 2 damage to target creature" — and its two halves have to be read
/// in two different places. The damage is read off a board with a 1/1 and a
/// 6/6 across the table, so "target creature" is lethal to exactly the one
/// that was named; the sacrifice is read where CR 601.2c and CR 601.2h put it,
/// with the target chosen while the Lunatic still stands and the four red
/// still float, and only afterwards is the card in its owner's graveyard and
/// the pool empty.
#[test]
fn barbarian_lunatic_sacrifices_itself_to_deal_two_damage_to_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[barbarian_lunatic(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[llanowar_elves(), rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lunatic = on_battlefield(&engine, p0, barbarian_lunatic()).expect("the Lunatic is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("a 1/1 across the table");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("and a 6/6 beside it");
    assert_eq!(pt(&engine, lunatic), (2, 1), "the body the card prints");
    assert_eq!(pt(&engine, elf), (1, 1), "two damage is lethal to this");
    assert_eq!(pt(&engine, wurm), (6, 6), "and not to that");

    // The {2}{R} is read off the pool and not off the untapped Mountains, so
    // the mana is floated before anything is claimed about the offer. The
    // Lunatic makes no mana of its own, so the pool is the Mountains exactly.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, three red — exactly the {{2}}{{R}} the line charges, \
         so the pool emptying below is a number and not a coincidence"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lunatic, 0)),
        "with the price floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, barbarian_lunatic(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elf) && options.contains(&wurm) && options.contains(&lunatic),
        "\"target creature\" is any creature on the battlefield — the two \
         across the table and the Lunatic itself: {options:?}"
    );

    // CR 601.2c names the target before CR 601.2h pays, so while the question
    // stands the Lunatic is still a permanent and the three red are still
    // floating.
    assert!(
        on_battlefield(&engine, p0, barbarian_lunatic()).is_some(),
        "the sacrifice is a cost, and a cost follows the target"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{2}}{{R}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");

    assert!(
        in_graveyard(&engine, p0, barbarian_lunatic()).is_some(),
        "\"Sacrifice this creature\" is the last thing paid, and it takes the \
         whole card"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{R}} went with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_some(),
        "the 6/6 the ability did not name is untouched"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the \
         player whose board it stood on"
    );
}
