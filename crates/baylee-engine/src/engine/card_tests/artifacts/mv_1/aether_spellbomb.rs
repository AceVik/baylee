//! `cards/artifacts/mv_1/aether_spellbomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aether Spellbomb prints two lines and both are a sacrifice: "{U},
/// Sacrifice this artifact: Return target creature to its owner's hand" and
/// "{1}, Sacrifice this artifact: Draw a card." Two copies stand on the
/// board because each line consumes the permanent that carries it, and the
/// pool is read before each claim — `legal.abilities` is filtered by
/// `can_afford`, which reads the pool and not the untapped lands, so a
/// bounce asserted off an empty pool would stay green whether the blue was
/// ever paid or not. The Elf across the table is what makes the target
/// worth reading: it is offered, the artifacts are not, and the card goes to
/// the hand of the seat that *owns* it rather than the one that aimed the
/// bounce.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn aether_spellbomb_bounces_a_creature_to_its_owners_hand_and_then_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                aether_spellbomb(),
                aether_spellbomb(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let bombs = all_on_battlefield(&engine, p0, aether_spellbomb());
    assert_eq!(bombs.len(), 2, "two copies, one per printed line");

    // The artifact makes no mana, so nothing has to be named as kept back:
    // `tap_all_mana` can only spend the three Islands and the Elf beside
    // them, and both are counted here rather than assumed away.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        3,
        "three Islands, three blue"
    );
    assert_eq!(
        pool.total(),
        4,
        "and the Elf's own {{G}}, because a mana creature is a mana route too"
    );

    // Ability 0: "{U}, Sacrifice this artifact: Return target creature to its
    // owner's hand."
    activate(&mut engine, p0, aether_spellbomb(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the bounce targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&bombs[0]) && !options.contains(&bombs[1]),
        "an artifact is no creature, and one of these is the source itself: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // sacrifice has not happened while this question is still open.
    assert!(
        on_battlefield(&engine, p0, aether_spellbomb()).is_some(),
        "the cost is paid after the target, not before it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that aimed the bounce"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, aether_spellbomb()).is_some(),
        "the Spellbomb was sacrificed to pay for it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "one of the three blue paid the {{U}}"
    );

    // Ability 1: "{1}, Sacrifice this artifact: Draw a card.", on the copy
    // that is still standing.
    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, aether_spellbomb(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the top card of the library went to the hand of the seat that spent it"
    );
    assert!(
        on_battlefield(&engine, p0, aether_spellbomb()).is_none(),
        "the second copy ate itself too"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and one more mana paid the {{1}}"
    );
}
