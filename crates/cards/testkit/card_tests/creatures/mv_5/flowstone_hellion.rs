//! `cards/creatures/mv_5/flowstone_hellion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "69ea4ef2-794d-4c76-904f-0435b0efd6a0"

/// Flowstone Hellion is a {4}{R} 3/3 with haste and "{0}: This creature gets
/// +1/-1 until end of turn" — one price pulling two ways, so the board has to
/// read both: two activations take it to (5, 1) and a third to (6, 0), which
/// CR 704.5f puts into the graveyard, where a mis-read "+1/+1" would still be
/// standing at (6, 6). Haste is played rather than read: the Hellion was cast
/// this turn, so CR 302.6 would keep it off the attackers list unless the
/// printed keyword is real, and its pumped five power is what the defending
/// seat actually loses.
#[test]
fn flowstone_hellion_trades_toughness_for_power_and_attacks_the_turn_it_arrives() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flowstone_hellion()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Mountains pay {4}{R} to the last drop, so the pool is read before
    // the cast and the {0} below is read against an empty one: the pump's whole
    // price is the ability itself and no mana at all.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains, five red"
    );
    cast_with_floating(&mut engine, p0, flowstone_hellion());
    pass_until(&mut engine, stack_is_empty);

    let hellion = on_battlefield(&engine, p0, flowstone_hellion()).expect("the Hellion resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, hellion), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, hellion).contains(KeywordSet::HASTE),
        "the printed haste reaches the permanent"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Hellion holds it");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{R}} spent every Mountain on the board"
    );
    assert!(
        legal.abilities.contains(&(hellion, 0)),
        "{{0}} is a price an empty pool pays, so the line is offered: {:?}",
        legal.abilities
    );

    // Two activations: the power climbs by two and the toughness falls by two,
    // which is the only reading that applies both printed numbers — the
    // alternative "+1/+1" would leave (5, 5) standing here.
    activate(&mut engine, p0, flowstone_hellion(), 0);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, flowstone_hellion(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, hellion),
        (5, 1),
        "+1/-1 twice: (3, 3) becomes (5, 1)"
    );

    // Haste, played: the Hellion arrived this turn, so CR 302.6 would keep it
    // out of the attackers list unless the keyword is real. Their Elf stands
    // across the table so that the block question is a real one.
    let blocks = attack_and_collect_blocks(&mut engine, hellion, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == elf && o.attackers.contains(&hellion)),
        "their untapped Elf is offered as a blocker for the Hellion that just \
         arrived, so the Hellion really is attacking: {blocks:?}"
    );
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the declare-blockers question, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");

    // Nothing blocked, so the pumped five power is what the defending seat
    // loses — and the pump is still running, because it lasts until end of turn.
    pass_until(&mut engine, |e| {
        e.state().players[1].life < 20
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().players[1].life,
        15,
        "an unblocked 5/1 attacking the turn it was cast"
    );
    assert_eq!(
        pt(&engine, hellion),
        (5, 1),
        "combat damage reads the same body the activations built"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf that was offered as a blocker was never declared as one"
    );

    // The third activation is a 6/0, and a creature with zero toughness is put
    // into its owner's graveyard by CR 704.5f — the half a "+1/+1" mis-reading
    // would not have shown.
    activate(&mut engine, p0, flowstone_hellion(), 0);
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, flowstone_hellion()).is_none()
    });
    assert!(
        in_graveyard(&engine, p0, flowstone_hellion()).is_some(),
        "one more +1/-1 pumps a 5/1 to 6/0, and a 6/0 is not a creature this \
         game keeps"
    );
}
