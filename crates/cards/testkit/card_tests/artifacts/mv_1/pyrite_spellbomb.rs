//! `cards/artifacts/mv_1/pyrite_spellbomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pyrite Spellbomb prints two activated abilities and each of them
/// sacrifices it: "{1}, Sacrifice this artifact: Draw a card" and "{R},
/// Sacrifice this artifact: It deals 2 damage to any target."
///
/// The scenario plays the damage half in the order the rules put it — the
/// target at CR 601.2c, the mana and the sacrifice at CR 601.2h — so while
/// the target question stands the bomb is still on the battlefield and the
/// {R} is still in the pool, and only afterwards is the artifact in its
/// owner's graveyard and a printed 1/1 across the table dead. "Any target"
/// (CR 115.4) is where the two option lists meet: one carries the Elf, the
/// other both seats, and the 20 life p1 keeps is the control that says the
/// two damage went to the creature that was named and not to the player
/// whose board it stood on. The drawing half is read off the same offer
/// rather than played, because whichever line resolves first eats the
/// artifact both of them cost.
#[test]
fn pyrite_spellbomb_eats_itself_for_two_damage_to_the_target_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[pyrite_spellbomb()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, elf), (1, 1), "a 1/1 for two damage to kill");

    // {1} out of two Mountains, which leaves exactly the {R} the other half
    // of the card charges — the same main phase, so CR 500.5 keeps it there.
    cast_from_hand(&mut engine, p0, pyrite_spellbomb());
    pass_until(&mut engine, stack_is_empty);
    let bomb = on_battlefield(&engine, p0, pyrite_spellbomb()).expect("the Spellbomb resolved");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the {{1}} is paid and one red is left floating for the {{R}}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bomb, 0)),
        "{{1}}, Sacrifice this artifact: Draw a card — offered with one mana \
         floating: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(bomb, 1)),
        "and {{R}}, Sacrifice this artifact: It deals 2 damage to any target \
         — ability 1 in the card def, behind the draw: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, pyrite_spellbomb(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, pyrite_spellbomb()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h), \
         so the bomb has not eaten itself yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{R}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    assert!(
        in_graveyard(&engine, p0, pyrite_spellbomb()).is_some(),
        "\"Sacrifice this artifact\" is the last thing paid, and it takes the \
         whole card"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} went with it"
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
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the \
         player whose board it stood on"
    );
}
