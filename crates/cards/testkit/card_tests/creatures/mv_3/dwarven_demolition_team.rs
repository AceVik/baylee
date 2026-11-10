//! `cards/creatures/mv_3/dwarven_demolition_team.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dwarven Demolition Team is a 1/1 Dwarf printing one line: "{T}: Destroy
/// target Wall." The scenario plays exactly that, and the board is built so
/// that both halves of the sentence have to come from the engine rather than
/// from the card file. The whole price is the Dwarf's own `{T}`, so the empty
/// pool is the point: the offer cannot be an affordability accident. And
/// "Wall" is a *subtype*, so an untargeted Llanowar Elves stands on each side
/// of the table as the control — a filter that had widened to any creature
/// would offer both of them.
#[test]
fn dwarven_demolition_team_taps_to_destroy_a_wall_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let wall = card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dwarven_demolition_team(), llanowar_elves()])
        .battlefield(1, &[wall, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dwarf = on_battlefield(&engine, p0, dwarven_demolition_team()).expect("the Dwarf is out");
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let their_wall = on_battlefield(&engine, p1, wall).expect("their Wall is out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // `{T}` and nothing else, so nothing has to be tapped first: a pool of
    // zero says the offer below is about the tap symbol and not about mana.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap, so no mana floats for this"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dwarf, 0)),
        "an untapped Dwarf on an empty pool is offered its one line: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dwarven_demolition_team(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Wall\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&their_wall),
        "\"target Wall\" reaches the Wall across the table: {options:?}"
    );
    assert!(
        !options.contains(&my_elves) && !options.contains(&their_elves),
        "a subtype and not \"target creature\": the Elves on either side are no \
         Wall: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "the Wall is the whole menu on this board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_wall],
            },
        )
        .expect("the Wall was one of the options it enumerated");
    // CR 601.2h: the {T} is the last step of the activation, so it is read
    // after the target question has been answered.
    assert!(is_tapped(&engine, dwarf), "{{T}} was the cost");
    assert!(
        !stack_is_empty(&engine),
        "destroying a permanent is no mana ability, so the ability is waiting"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, wall).is_none(),
        "the targeted Wall left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, wall).is_some(),
        "and went to its owner's graveyard, which is the seat that controlled it"
    );
    assert!(
        on_battlefield(&engine, p0, dwarven_demolition_team()).is_some(),
        "the Dwarf is untouched — the ability destroys its target and nothing else"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "on either side of the table"
    );
}
