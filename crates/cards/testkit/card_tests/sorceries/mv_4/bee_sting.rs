//! `cards/sorceries/mv_4/bee_sting.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bee Sting is a {3}{G} sorcery under `Coverage::Implemented` that deals 2 damage to any target.
/// Under `CR 115.4`, "any target" presents creatures and players together as legal targets.
/// When aimed at the opposing player, 2 damage is dealt upon resolution, reducing their life total from 20 to 18.
/// Creatures on the battlefield remain untouched when a player is targeted.
#[test]
fn bee_sting_deals_two_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[bee_sting()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, bee_sting()).expect("Bee Sting is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{3}}{{G}}"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests produce four green mana"
    );
    cast_with_floating(&mut engine, p0, bee_sting());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(player, p0, "caster selects target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&elf),
        "creatures are offered for \"any target\": {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "players are offered for \"any target\": {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting player 1 is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "opponent took 2 damage from Bee Sting"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "caster life total unchanged"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "unselected Elf remains on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, bee_sting()).is_some(),
        "Bee Sting went to graveyard upon resolution"
    );
}
