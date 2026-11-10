//! `cards/sorceries/mv_7/vampiric_feast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1980ca2e-a415-4de1-ac30-7055507e82a2"

/// Vampiric Feast — {5}{B}{B} sorcery: "Vampiric Feast deals 4 damage to any
/// target and you gain 4 life." One card, two casts, and the two halves of
/// "any target" (CR 115.4) are played in the shapes they come in: the first
/// Feast is aimed at a creature across the table, where four damage kills a
/// printed 1/1 while the damage's owner keeps all twenty life, and the second
/// at that creature's controller, where the damage lands on a player instead.
/// The life gain is read in both — four to the caster whichever target was
/// named, which is the pairing no reading of the card file can confirm.
#[test]
#[allow(clippy::too_many_lines)]
fn vampiric_feast_damages_any_target_it_names_and_gains_its_caster_four_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Fourteen Swamps is two casts of {5}{B}{B}, and the whole scenario plays
    // inside one main phase, so CR 500.5 never empties the pool between them.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 14])
        .hand(0, &[vampiric_feast(), vampiric_feast()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, elf), (1, 1), "a 1/1 for four damage to kill");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        14,
        "fourteen Swamps, fourteen black"
    );

    // First cast: "any target" in its object shape. The Elf is on the menu,
    // the players are the other half of the same prompt, and neither side of
    // the choice has been spent while the question stands (CR 601.2c before
    // CR 601.2h).
    cast_with_floating(&mut engine, p0, vampiric_feast());
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
    assert_eq!(player, p0, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "a creature is one half of \"any target\": {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        14,
        "the cost is the last step of the cast, so nothing is spent yet"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature was one of the options it enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the {{5}}{{B}}{{B}} left the pool, and the second cast is still funded"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "four damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "and its caster gained 4 life for a creature, not for a hit on its own head"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the player whose board it stood on"
    );

    // Second cast: the same phrase in its player shape, off the mana the first
    // one left behind.
    cast_with_floating(&mut engine, p0, vampiric_feast());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the second cast draws on exactly the seven the first one left"
    );
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it again");
    assert!(
        player_options.contains(&p1),
        "an opponent is the other half of \"any target\": {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the player was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "four damage to the player it named, and not to the creature that is already gone"
    );
    assert_eq!(
        engine.state().players[0].life,
        28,
        "\"you gain 4 life\" fires whichever half of \"any target\" was chosen"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both feasts came out of the fourteen Swamps and no mana is left floating"
    );
    assert!(
        in_graveyard(&engine, p0, vampiric_feast()).is_some(),
        "and the sorcery itself is in its owner's graveyard, not still on the stack"
    );
}
