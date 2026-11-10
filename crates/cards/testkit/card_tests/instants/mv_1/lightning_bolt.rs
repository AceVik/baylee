//! `cards/instants/mv_1/lightning_bolt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lightning Bolt prints a line: "{R} — Instant: Lightning Bolt deals 3
/// damage to any target." Exactly that is played twice here, because
/// `any target` (CR 115.4) means both lists of *one* target question:
/// the first Bolt kills the printed 1/1 Elf on the battlefield, the
/// second goes to the player. The 20 life after the creature hit are the
/// control — they rule out that the damage went to both targets at once
/// —, and the two Mountains are the entire cost of both spells in one
/// main phase (CR 500.5), so that "two red" before the first `apply`
/// is a statement about the pool and not about untapped lands.
#[test]
fn lightning_bolt_deals_three_to_a_creature_or_a_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene Main"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf on the table");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "ein gedruckter 1/1 für drei Schaden"
    );

    // Both Mountains, both red mana: `legal.castable` reads the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, two red, and nothing else on the board"
    );

    cast_with_floating(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("der Blitz zielt, got {:?}", engine.pending())
    };
    assert_eq!(
        (player, min, max),
        (p0, 1, 1),
        "one target, and the caster chooses it"
    );
    assert!(
        options.contains(&elf),
        "the creature above the table is a target: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: `any target` counts players in the same choice: {player_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage to a 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the player got no point from it"
    );

    // The second Bolt at the other target: the same line, and the damage
    // lands where the response carries it.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("auch der zweite Blitz zielt, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a target for `any target`: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "exactly three life, and no more"
    );
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "the Blitz landed in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both {{R}} are paid, so the cost of the second Bolt was real"
    );
}
