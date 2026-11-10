//! `cards/creatures/mv_3/prodigal_sorcerer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Prodigal Sorcerer is a `{2}{U}` 1/1 whose entire text is "`{T}`: This
/// creature deals 1 damage to any target." Two are seated rather than cast —
/// a Sorcerer cast this turn is summoning sick (CR 302.6) and could not tap
/// until its controller's next turn, so the one doing the pinging has to be a
/// permanent that has been under that seat's control since the turn began —
/// and between them they take both halves of *any target* (CR 115.4): one
/// kills a printed 1/1 Elf across the table, the other takes exactly one life
/// off the opponent. The Elf is what makes the object half a real choice, and
/// the opponent's untouched life total after the first ping says the damage
/// landed where it was aimed and not on the seat that Elf stood on.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn prodigal_sorcerer_taps_to_deal_one_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[prodigal_sorcerer(), prodigal_sorcerer()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wizards = all_on_battlefield(&engine, p0, prodigal_sorcerer());
    assert_eq!(wizards.len(), 2, "two Sorcerers are seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, wizards[0]), (1, 1), "a printed 1/1 body");

    // The whole price of the line is the Sorcerer's own {T}, so no mana has to
    // be made before the offer can be read.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        wizards.iter().all(|id| legal.abilities.contains(&(*id, 0))),
        "each untapped Sorcerer offers its one line: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, prodigal_sorcerer(), 0);
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
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    assert!(
        wizards.iter().all(|id| !is_tapped(&engine, *id)),
        "targets are named before costs are paid (CR 601.2c, then CR 601.2h), \
         so no Sorcerer has tapped itself yet"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");
    assert_eq!(
        wizards
            .iter()
            .copied()
            .filter(|id| is_tapped(&engine, *id))
            .count(),
        1,
        "exactly one Sorcerer paid the {{T}}, which is the whole price"
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
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named, not to the seat whose \
         board it stood on"
    );

    // The other half of "any target": a player is a legal target too, off the
    // second Sorcerer, which nothing has tapped yet.
    assert_eq!(
        wizards
            .iter()
            .copied()
            .filter(|id| !is_tapped(&engine, *id))
            .count(),
        1,
        "the other Sorcerer is still standing"
    );
    activate(&mut engine, p0, prodigal_sorcerer(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal target as well: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a face was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        wizards.iter().all(|id| is_tapped(&engine, *id)),
        "the second Sorcerer paid its own {{T}} as well"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage\" — one, and not two"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage landed on the seat it was aimed at, not on the one that \
         aimed it"
    );
}
