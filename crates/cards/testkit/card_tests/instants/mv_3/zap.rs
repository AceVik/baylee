//! `cards/instants/mv_3/zap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zap — {2}{R} instant: "Zap deals 1 damage to any target. Draw a card."
/// Both printed sentences are read in one cast, and the target question is
/// where CR 115.4 is checked: `options` carries the Elf across the table and
/// `player_options` both seats, because "any target" is one choice over
/// objects and players together. The damage is read on a life total rather
/// than on a body, since 20 to 19 is exactly the printed one where a dead 1/1
/// would only say "at least one", and the draw is read off the library and the
/// hand together so an emptied library could not stand in for a card.
#[test]
#[allow(clippy::too_many_lines)]
fn zap_deals_one_damage_to_the_target_it_names_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[zap()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `legal.castable` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands, so the claim is made before anything is
    // tapped: {{2}}{{R}} is not payable on an empty pool.
    let card = in_hand(&engine, p0, zap()).expect("Zap is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{R}}, so the instant is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, zap());

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
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nothing has happened yet: the target is chosen before anything resolves"
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

    assert!(
        !stack_is_empty(&engine),
        "an instant is a spell and waits on the stack rather than resolving on announcement"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage is the resolution, not the cost"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — exactly one, read on the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        in_graveyard(&engine, p0, zap()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved, so the one point went where it was aimed"
    );
}
