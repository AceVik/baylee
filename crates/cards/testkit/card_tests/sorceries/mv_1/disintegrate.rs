//! `cards/sorceries/mv_1/disintegrate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Disintegrate: "deals X damage to any target. If it's a creature, it
/// can't be regenerated this turn, and if it would die this turn, exile it
/// instead." A shielded creature dies anyway, and lands in exile, not the
/// graveyard.
#[test]
fn disintegrate_exiles_a_shielded_creature_instead_of_letting_it_die() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), quiet_creature()])
        .hand(0, &[disintegrate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(elf)
        .expect("seated")
        .regeneration_shields = 1;

    cast_from_hand(&mut engine, p0, disintegrate());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "2 damage kills a 1-toughness Elf"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_none(),
        "\"exile it instead\" — not the graveyard"
    );
    let exiled = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(p0))
        .iter()
        .any(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == quiet_creature()))
        });
    assert!(exiled, "the Elf is in exile");
}

/// "X damage to any target": a player is a target too. X = 3 at the opponent
/// costs them 3 life and exiles nothing; the creature rider is conditional on
/// the target being a creature, so the opponent's Elf, dying later in the same
/// turn, goes to the graveyard as usual.
#[test]
fn disintegrate_to_a_player_deals_x_and_leaves_creatures_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[disintegrate()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = engine.state().players[1].life;

    cast_from_hand(&mut engine, p0, disintegrate());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "\"any target\" includes the opponent: {player_options:?}"
    );
    assert!(
        options.len() == 1,
        "and the Elf, the one creature: {options:?}"
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
        before - 3,
        "X = 3 damage to the player"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("untouched");
    kill(&mut engine, elf);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the exile rider belongs to a creature that was targeted, not to this turn at large"
    );
}
