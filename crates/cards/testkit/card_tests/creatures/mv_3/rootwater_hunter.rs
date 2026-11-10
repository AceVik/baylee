//! `cards/creatures/mv_3/rootwater_hunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rootwater Hunter — {2}{U} 1/1 Merfolk: "{T}: This creature deals 1 damage
/// to any target."
///
/// "Any target" (CR 115.4) is the whole card: one damage that may be aimed at
/// a creature *or* at a player, and only playing both of them names it. So two
/// Hunters are cast — one kills a 1/1 Elf across the table, the other is aimed
/// at the opponent's face — and each activation is read where the rules put its
/// price: the target at CR 601.2c while the Hunter is still untapped, the tap
/// symbol at CR 601.2h after the answer. The turn in between is what the {T}
/// costs a creature that has just arrived (CR 302.6).
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn rootwater_hunter_pings_a_creature_or_a_player_and_taps_for_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(29, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[rootwater_hunter(), rootwater_hunter()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two {2}{U} spells out of six Islands, both inside the one main phase, so
    // the pool CR 500.5 empties never has to carry anything across a step.
    assert_eq!(tap_all_mana(&mut engine, p0), 6, "six Islands, six mana");
    cast_with_floating(&mut engine, p0, rootwater_hunter());
    pass_until(&mut engine, |e| {
        all_on_battlefield(e, p0, rootwater_hunter()).len() == 1
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the first Hunter took {{2}}{{U}} out of the pool"
    );
    cast_with_floating(&mut engine, p0, rootwater_hunter());
    pass_until(&mut engine, |e| {
        all_on_battlefield(e, p0, rootwater_hunter()).len() == 2
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the second took the rest of it"
    );

    let hunters = all_on_battlefield(&engine, p0, rootwater_hunter());
    assert_eq!(hunters.len(), 2, "both Hunters resolved");
    let (first, second) = (hunters[0], hunters[1]);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is standing");
    assert_eq!(pt(&engine, first), (1, 1), "a printed 1/1 either way");

    // A creature that entered this turn cannot pay a {T} (CR 302.6), so the
    // whole card is unofferable until its controller's next turn — which is
    // why the two activations below wait for one.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == first),
        "summoning sick: its {{T}} is not a price it may pay yet: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // ---- the first Hunter: 1 damage to a creature ------------------------
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == first)
        .expect("untapped and past summoning sickness, the line is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

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
    assert_eq!(
        (player, min, max),
        (p0, 1, 1),
        "the activating seat, and exactly one target"
    );
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
        "CR 601.2c before CR 601.2h: the target is named while the tap is \
         still unpaid"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");
    assert!(
        is_tapped(&engine, first),
        "{{T}} is the whole price, and it is paid after the target"
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
        "the damage went to the creature that was named, not to its controller"
    );

    // ---- the second Hunter: 1 damage to a player -------------------------
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == second)
        .expect("the other Hunter is the one still untapped");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "a player is a target of its own kind here (CR 115.4): {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was one of the players it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"1 damage to any target\" is a player's life total too"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the seat that was aimed at"
    );
    assert!(
        is_tapped(&engine, first) && is_tapped(&engine, second),
        "both Hunters paid their {{T}}, and neither came back untapped"
    );
}
