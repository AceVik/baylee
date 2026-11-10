//! `cards/sorceries/mv_6/desert_twister.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert Twister — {4}{G}{G} sorcery: "Destroy target permanent."
///
/// The load-bearing word is *permanent*, so the board holds one of each thing
/// that word could be misread as: a Forest of mine (a land, which "target
/// creature" would drop), the Elves beside it (a creature, which a noncreature
/// filter would drop) and a Forest plus a Sol Ring across the table, which
/// "target permanent *you control*" would never offer. The empty
/// `player_options` is the other half of the reading: a seat is not a
/// permanent, so the menu cannot be the "any target" one (CR 115.4). Seven
/// green come off six Forests and the Elves' own `{T}: Add {G}`, six of them
/// leave the pool only when the target question is answered (CR 601.2c before
/// CR 601.2h), and the named land is then read in its owner's graveyard with
/// every other permanent still standing.
#[test]
#[allow(clippy::too_many_lines)]
fn desert_twister_destroys_any_permanent_on_either_side_of_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![forest(); 6];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[desert_twister()])
        .battlefield(1, &[quiet_artifact(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_land = on_battlefield(&engine, p0, forest()).expect("one of my Forests is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // Every source on the board, and six green is not enough: the six Forests
    // and the Elves' own printed `{T}: Add {G}` make seven (rule 11).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "six Forests and the Elves' own {{G}}"
    );

    cast_with_floating(&mut engine, p0, desert_twister());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one permanent, and the spell asks once");
    assert!(
        player_options.is_empty(),
        "a player is no permanent: \"target permanent\" names no seat, which is \
         what tells it from \"any target\": {player_options:?}"
    );
    assert!(
        options.contains(&my_land),
        "a land is a permanent, so one of my own is on the menu: {options:?}"
    );
    assert!(
        options.contains(&my_elf),
        "and a creature of mine is too: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "\"target permanent\" is not \"target permanent you control\": the \
         Forest across the table is a legal target: {options:?}"
    );
    assert!(
        options.contains(&their_rock),
        "and so is the artifact standing beside it: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "CR 601.2c before CR 601.2h: no cost is paid while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("their Forest was one of the options the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "six green paid {{4}}{{G}}{{G}} out of the seven the board made"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the permanent the spell named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        in_graveyard(&engine, p0, desert_twister()).is_some(),
        "the sorcery itself resolved and was put into its caster's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        6,
        "and none of mine did either: one target, one permanent destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature that was on the menu but not chosen is still standing"
    );
}
