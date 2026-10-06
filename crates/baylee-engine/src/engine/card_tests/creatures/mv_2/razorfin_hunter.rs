//! `cards/creatures/mv_2/razorfin_hunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Razorfin Hunter — {U}{R}, 1/1 Merfolk Goblin: "{T}: This creature deals 1
/// damage to any target."
///
/// Cast for exactly the {U}{R} an Island and a Mountain make, then played on
/// the turn after: a creature that entered this turn has not been under its
/// controller's control since the beginning of one and could not pay its own
/// {T} (CR 302.6). The whole price of the ability is that tap and no mana, so
/// the empty pool leaves the life total as the only thing that can move —
/// aimed at the player across the table it is one damage and one damage only,
/// while the 1/1 Elf sitting on the same menu as an object option never moves.
#[test]
fn razorfin_hunter_taps_for_one_damage_to_the_target_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), mountain()])
        .hand(0, &[razorfin_hunter()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, razorfin_hunter());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, razorfin_hunter()).is_some()
    });
    let hunter = on_battlefield(&engine, p0, razorfin_hunter()).expect("the Hunter resolved");
    assert_eq!(pt(&engine, hunter), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "an Island and a Mountain pay {{U}}{{R}} exactly"
    );

    // A turn cycle, so the Hunter is no longer summoning sick and its {T} may
    // be paid at all. The opponent's Elf untaps and does nothing.
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, hunter),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating: the whole price of the ability is its own {{T}}"
    );
    let (mine_before, theirs_before) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );

    activate(&mut engine, p0, razorfin_hunter(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated aims it");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        !is_tapped(&engine, hunter),
        "CR 601.2c comes before CR 601.2h: the tap is the last step, so the \
         Hunter is still standing while the question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target for `any target`");
    assert!(
        is_tapped(&engine, hunter),
        "answering the target finished the activation, and the price was {{T}}"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        theirs_before - 1,
        "\"deals 1 damage\" — one, on a board where a two would have read as two"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine_before,
        "the damage belongs to the target that was named and not to both seats"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the object the ability did not name never moved"
    );
}
