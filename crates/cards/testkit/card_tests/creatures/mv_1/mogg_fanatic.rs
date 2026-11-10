//! `cards/creatures/mv_1/mogg_fanatic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mogg Fanatic prints one line — "Sacrifice this creature: It deals 1 damage
/// to any target" — and one activation reads all three of its parts. The
/// target question offers both kinds of target at once, a creature as an
/// object and an opponent as a player (CR 115.4), which is what "any" means
/// and what a client that only ever offered permanents would lose. The
/// sacrifice is still unpaid while that question stands, because targets are
/// chosen before costs (CR 601.2c, then 601.2h), and the one damage only
/// lands on resolution — leaving exactly one creature, the one nobody named,
/// standing across the table.
#[test]
fn mogg_fanatic_sacrifices_itself_to_deal_one_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[mogg_fanatic()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {R} off the one Mountain, so the pool is empty before the ability is
    // pressed: its whole price is the body the card prints.
    cast_from_hand(&mut engine, p0, mogg_fanatic());
    pass_until(&mut engine, stack_is_empty);
    let fanatic = on_battlefield(&engine, p0, mogg_fanatic()).expect("the Fanatic resolved");
    assert_eq!(pt(&engine, fanatic), (1, 1), "the printed 1/1 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} is spent, and the ability asks for no mana at all"
    );

    activate(&mut engine, p0, mogg_fanatic(), 0);
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
            "\"any target\" is a target question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one target, and it may be any");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        options.contains(&elves),
        "a creature is a legal target: {options:?}"
    );
    assert!(
        player_options.contains(&p1),
        "and so is a player — CR 115.4's *any target* is both lists, not \
         just the permanents: {player_options:?}"
    );
    assert_eq!(engine.state().players[1].life, 20, "nothing dealt yet");
    assert!(
        on_battlefield(&engine, p0, mogg_fanatic()).is_some(),
        "and the sacrifice is the *last* step (CR 601.2h): while the \
         question stands, the Goblin is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p1],
            },
        )
        .expect("the player the question offered is an answer to it");

    assert!(
        on_battlefield(&engine, p0, mogg_fanatic()).is_none(),
        "paying the cost took the Fanatic off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, mogg_fanatic()).is_some(),
        "and put it in its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability itself is on the stack, not yet resolved"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "so no damage has been dealt yet"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"it deals 1 damage to any target\": one, off the top of twenty"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the creature the ability was not aimed at is untouched"
    );
}
