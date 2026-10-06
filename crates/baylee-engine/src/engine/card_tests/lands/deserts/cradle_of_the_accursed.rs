//! `cards/lands/deserts/cradle_of_the_accursed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cradle of the Accursed: "{T}: Add {C}." / "{3}, {T}, Sacrifice this land: Create a 2/2 black Zombie creature token."
/// Under `Coverage::Partial`, the Zombie token creation ability is omitted.
/// Activating the implemented ability taps the land for {C} and adds colorless mana to the pool.
#[test]
fn cradle_of_the_accursed_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(117, forest())
        .battlefield(0, &[cradle_of_the_accursed()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, cradle_of_the_accursed()).expect("Cradle deployed");
    activate(&mut engine, p0, cradle_of_the_accursed(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Cradle of the Accursed: "{3}, {T}, Sacrifice this land: Create a 2/2
/// black Zombie creature token. Activate only as a sorcery." Two halves
/// need a witness. The timing is read on the **opponent's** turn, where the
/// same board and the same three mana offer nothing — CR 602.5d is about
/// when priority is held and not about what the board can pay. And the land
/// is gone the moment the ability is announced, because a sacrifice is a
/// cost (CR 601.2h): the Zombie arriving later is what says the two are not
/// one step.
#[test]
fn cradle_of_the_accursed_sells_itself_for_a_zombie_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9102, forest())
        .battlefield(0, &[cradle_of_the_accursed(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent's main phase, with everything this seat needs standing.
    // The active player passes first, because the question is what *this*
    // seat is offered while holding priority on somebody else's turn.
    reach_their_main_phase(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("the active player passes");
    let cradle =
        on_battlefield(&engine, p0, cradle_of_the_accursed()).expect("the Desert is seated");
    tap_mana_except(&mut engine, p0, cradle);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the three Forests pay the {{3}} on either turn"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cradle, 1)),
        "\"Activate only as a sorcery\" — not on the opponent's turn: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(cradle, 0)),
        "while the mana ability, which prints no such line, is offered"
    );

    walk_to_own_main(&mut engine, p0);
    let cradle = on_battlefield(&engine, p0, cradle_of_the_accursed()).expect("still seated");
    tap_mana_except(&mut engine, p0, cradle);
    activate(&mut engine, p0, cradle_of_the_accursed(), 1);
    assert!(
        on_battlefield(&engine, p0, cradle_of_the_accursed()).is_none(),
        "the sacrifice is a cost, paid as the ability is announced"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Zombie is the resolution, not the announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 1, "one activation, one Zombie");
    let zombie = made[0];
    assert_eq!(pt(&engine, zombie), (2, 2), "the printed 2/2");
    assert!(types(&engine, zombie).contains(TypeSet::CREATURE));
    let printed = engine
        .state()
        .object(zombie)
        .expect("the Zombie is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Zombie");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Black),
        "\"a 2/2 black Zombie\""
    );
}
