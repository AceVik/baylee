//! `cards/creatures/mv_4/shock_troops.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shock Troops prints one line — "Sacrifice this creature: It deals 2 damage
/// to any target" — and the whole price is the creature itself: no mana and no
/// tap symbol. The board therefore carries no land at all, which is what makes
/// "it is offered on an empty pool" an exact claim rather than a coincidence,
/// and the target is read on the other side of the table: two damage kills a
/// printed 1/1 (CR 704.5g) while the seat that owns it stays at twenty, which
/// is what separates the object half of "any target" from the player half the
/// same choice enumerates (CR 115.4). The sacrifice is the price, so it is read
/// where a cost lands — its owner's graveyard, with the ability still on the
/// stack — and not as something the resolution did.
#[test]
fn shock_troops_sacrifices_itself_for_two_damage_to_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shock_troops()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troops = on_battlefield(&engine, p0, shock_troops()).expect("the Troops are out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, troops), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land is in play at all, so the price is the creature and nothing else"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(troops, 0)),
        "the one line the card prints costs no mana, so it is offered on an \
         empty pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, shock_troops(), 0);
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
        on_battlefield(&engine, p0, shock_troops()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h), \
         so the Troops are still standing while the question is open"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    assert!(
        in_graveyard(&engine, p0, shock_troops()).is_some(),
        "\"Sacrifice this creature\" is the whole price, so the card is in its \
         owner's graveyard"
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
