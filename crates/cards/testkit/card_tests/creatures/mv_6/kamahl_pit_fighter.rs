//! `cards/creatures/mv_6/kamahl_pit_fighter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kamahl, Pit Fighter is a 6/1 legendary with haste and "{T}: Kamahl deals 3
/// damage to any target", and the two printed lines are only worth anything
/// together: a creature that entered this turn may not pay a {T} cost
/// (CR 302.6) unless it has haste (CR 702.10c), so the offer the engine
/// publishes on the very turn he was cast *is* the reading of the keyword.
/// The damage is aimed at the opponent's face, which is where "any target"
/// (CR 115.4) shows both halves of its menu — the Elf across the table and
/// both seats — while the creature nobody named stays standing.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn kamahl_pit_fighter_taps_the_turn_he_arrives_for_three_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[kamahl_pit_fighter()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Mountains are exactly {4}{R}{R}, and `cast_from_hand` taps every one
    // of them: the pool the ability is read against afterwards is empty, so the
    // only price it can be paying is its own tap symbol.
    cast_from_hand(&mut engine, p0, kamahl_pit_fighter());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let kamahl = on_battlefield(&engine, p0, kamahl_pit_fighter()).expect("Kamahl resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, kamahl), (6, 1), "the body the card prints");
    assert!(
        keywords(&engine, kamahl).contains(KeywordSet::HASTE),
        "the printed haste reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the six Mountains are spent, so nothing is floating toward a cost"
    );

    // The claim haste makes: the creature arrived this turn and its {T} ability
    // is offered anyway. A printing without the keyword would withhold the line
    // here rather than refuse it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(kamahl, 0)),
        "haste lets him pay a {{T}} the turn he arrives: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, kamahl_pit_fighter(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // {T} is still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, kamahl),
        "the {{T}} is the last step of the activation, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert!(is_tapped(&engine, kamahl), "{{T}} was the whole price");
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing has happened yet: the damage is the resolution"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "\"Kamahl deals 3 damage\" — three, and not a point per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the ability did not name never moved"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and still carries its printed body"
    );
    assert!(
        on_battlefield(&engine, p0, kamahl_pit_fighter()).is_some(),
        "an activated ability costs the creature nothing but its tap"
    );
}
