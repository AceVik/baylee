//! `cards/creatures/mv_3/vulshok_sorcerer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vulshok Sorcerer is a 1/1 for {1}{R}{R} with haste and "{T}: This
/// creature deals 1 damage to any target". Both halves are played in one
/// first main phase: the Sorcerer pings on the turn it arrives, which is the
/// whole of what the haste line buys (CR 302.6), and a second copy aims the
/// same line at a player instead of at a creature. The printed 1/1 across
/// the table and the 20 life beside it make the damage exact — one point
/// kills a 1/1 and leaves its controller untouched, and one point off a life
/// total is a number no other source on this board could have produced.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vulshok_sorcerer_pings_on_the_turn_it_arrives_and_reaches_creatures_and_players_alike() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[vulshok_sorcerer(), vulshok_sorcerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Mountains are exactly what two {1}{R}{R} casts cost, and they are
    // tapped before anything is claimed about the offer: `legal` is filtered
    // by `can_afford`, which reads the pool and not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains, six red"
    );

    cast_with_floating(&mut engine, p0, vulshok_sorcerer());
    pass_until(&mut engine, stack_is_empty);
    let first = on_battlefield(&engine, p0, vulshok_sorcerer()).expect("the Sorcerer resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        keywords(&engine, first).contains(KeywordSet::HASTE),
        "the printed haste line reaches the permanent"
    );

    // Ability 0 is the only activated line the card prints, and it is offered
    // on the turn the creature arrived — which is what haste is for (CR 302.6).
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the Sorcerer's controller holds it");
    assert!(
        legal.abilities.contains(&(first, 0)),
        "{{T}}: 1 damage to any target, on a creature cast this turn: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, vulshok_sorcerer(), 0);
    let Pending::ChooseTargets {
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
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    assert!(
        !is_tapped(&engine, first),
        "CR 601.2c before CR 601.2h: the {{T}} is the last step of the activation"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");
    assert!(
        is_tapped(&engine, first),
        "the tap symbol is the whole price the ability charges"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage went to the creature that was named, not to its controller"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the ping cost no mana: the pool is exactly where the {{1}}{{R}}{{R}} left it"
    );

    // The other half of "any target": the same line aimed at a player. The
    // second copy is the one still untapped, so nothing about the first
    // Sorcerer's tap is being read a second time below.
    cast_with_floating(&mut engine, p0, vulshok_sorcerer());
    pass_until(&mut engine, stack_is_empty);
    let shooters = all_on_battlefield(&engine, p0, vulshok_sorcerer());
    assert_eq!(shooters.len(), 2, "both Sorcerers are on the table");
    let second = *shooters
        .iter()
        .find(|id| !is_tapped(&engine, **id))
        .expect("one of the two is still untapped");

    activate(&mut engine, p0, vulshok_sorcerer(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a target the question offers: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player the question enumerated is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\": one point, to the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both casts paid and neither ping did: the pool is empty"
    );
    assert!(
        is_tapped(&engine, second),
        "and the second Sorcerer paid its own {{T}}"
    );
}
