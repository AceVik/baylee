//! `cards/creatures/mv_3/zuran_spellcaster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zuran Spellcaster is a `{2}{U}` 1/1 whose entire text is "`{T}`: This
/// creature deals 1 damage to any target."
///
/// The scenario casts it on turn one and then waits a whole turn cycle,
/// because the printed cost is the creature's own tap symbol and CR 302.6
/// withholds it until the untap step has been and gone — a test that pressed
/// it on the arrival turn would be reading summoning sickness and calling it
/// the card. The one damage is then read off a printed 1/1 across the table,
/// which dies to it, while its controller's life stays at twenty: an ability
/// that had turned the damage at the player, or dealt two, would satisfy the
/// graveyard count just as well. "Any target" is the offer itself, which
/// carries creatures and players in one choice (CR 115.4).
#[test]
fn zuran_spellcaster_taps_to_deal_one_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[zuran_spellcaster()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{U} out of the three Islands; the creature arrives as the 1/1 it
    // prints, untapped, and the Islands are the only mana on the board.
    cast_from_hand(&mut engine, p0, zuran_spellcaster());
    pass_until(&mut engine, stack_is_empty);
    let wizard =
        on_battlefield(&engine, p0, zuran_spellcaster()).expect("the Spellcaster resolved");
    assert_eq!(pt(&engine, wizard), (1, 1), "a printed 1/1");
    assert!(
        !is_tapped(&engine, wizard),
        "nothing tapped it to pay for itself"
    );

    // CR 302.6. The whole price of the ability is its own {T} and nothing
    // else, so `can_afford` has nothing to refuse here: the only thing that
    // can keep the line out of the offer this turn is the summoning sickness.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(wizard, 0)),
        "a creature's {{T}} ability is offered only once it has been under its \
         controller's control since the turn began: {:?}",
        legal.abilities
    );

    // A whole turn cycle, which is exactly what the printed tap symbol asks
    // for — and the Elf across the table is still a live 1/1 at the end of it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is still across the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1, and one damage is lethal"
    );
    assert!(
        !is_tapped(&engine, wizard),
        "the untap step stood the Spellcaster back up"
    );

    activate(&mut engine, p0, zuran_spellcaster(), 0);
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
    assert_eq!((min, max), (1, 1), "one damage, one target");
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    assert!(
        is_tapped(&engine, wizard),
        "{{T}} is the whole of the price"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature it named left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named, not to its controller"
    );
}
