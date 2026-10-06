//! `cards/sorceries/mv_2/volcanic_hammer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volcanic Hammer is a `{1}{R}` sorcery printing one sentence: "Volcanic
/// Hammer deals 3 damage to any target." The card is played twice off four
/// Mountains in one main phase, because "any target" is two lists in one
/// choice (CR 115.4) and a single cast can only exercise one of them: the
/// first Hammer is aimed at the opponent, whose life total reads the printed
/// number exactly (20 → 17, not 2 and not 4), and the second at the Elf
/// across the table, which dies while that same life total does not move
/// again.
///
/// Every bystander is a control. The Elf is standing while the first target
/// question is open — the offer carries it beside both players, and answering
/// with a player has to leave it where it is — and the second cast is paid
/// with the two red the first one left floating in the same phase (CR 500.5)
/// rather than with a source tapped for it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn volcanic_hammer_deals_three_to_a_player_and_then_to_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(311, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[volcanic_hammer(), volcanic_hammer()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, their_elf), (1, 1), "a printed 1/1");

    // Both casts come out of the same pool: four Mountains, four red, and a
    // pool that survives until the step ends (CR 500.5).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and no other mana source on this board"
    );

    cast_with_floating(&mut engine, p0, volcanic_hammer());
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
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "one target, and the card asks once");
    assert!(
        options.contains(&their_elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p1) && player_options.contains(&p0),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "3 damage to a player is exactly three life"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing was dealt to the seat that cast it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the Hammer did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, volcanic_hammer()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "the {{1}}{{R}} came out of the four red, and the two left are the \
         second cast's"
    );

    // The other half of "any target", off the same floating two.
    cast_with_floating(&mut engine, p0, volcanic_hammer());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&their_elf),
        "the same Elf is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the creature was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the second Hammer was aimed at the creature, so the life total does \
         not move again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the two red the first cast left behind paid for it"
    );
}
