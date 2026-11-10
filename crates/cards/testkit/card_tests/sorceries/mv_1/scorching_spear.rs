//! `cards/sorceries/mv_1/scorching_spear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scorching Spear is one printed line — "Scorching Spear deals 1 damage to
/// any target" — and "any target" (CR 115.4) is the whole of what a board can
/// hold it to: one question, one choice, with the permanents and the seats
/// arriving in the same `ChooseTargets`. The two copies in hand are aimed at
/// one of each because that pair is what pins the answer's list down — the Elf
/// across the table dies with its controller's life untouched, and then the
/// seat itself loses the life while every creature on the board still stands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn scorching_spear_deals_one_damage_to_a_creature_or_a_player_and_to_nothing_else() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .hand(0, &[scorching_spear(), scorching_spear()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, theirs), (1, 1), "a printed 1/1 for one damage");

    // {R} off the two Mountains. The target is named before the cost is paid
    // (CR 601.2c, then CR 601.2h), so the spell is already on the stack while
    // this question stands.
    cast_from_hand(&mut engine, p0, scorching_spear());
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
    assert_eq!(player, p0, "the caster does the aiming");
    assert_eq!((min, max), (1, 1), "one target, and the spell requires one");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"any target\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: players are the other half of the same choice: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .expect("the Elf across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the spear did not name never moved"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature, not to the seat whose board it stood on"
    );

    // The same question again, answered out of the other half of the list.
    cast_from_hand(&mut engine, p0, scorching_spear());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "the second spear asks the same choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "\"any target\" reaches the seat itself: {player_options:?}"
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
        19,
        "one damage to the player who was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the life belongs to that player, not to the caster"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "no creature was named the second time, so none moved"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "both spears resolved and went to their owner's graveyard"
    );
}
