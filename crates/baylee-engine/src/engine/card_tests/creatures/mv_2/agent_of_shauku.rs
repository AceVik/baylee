//! `cards/creatures/mv_2/agent_of_shauku.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Agent of Shauku — {1}{B}, a 1/1 — prints one line: "{1}{B}, Sacrifice a
/// land: Target creature gets +2/+0 until end of turn."
///
/// One activation asks both halves of that price, and the order they arrive in
/// is the rules rather than a convenience: the target is announced first
/// (CR 601.2c) and the cost is paid last (CR 601.2h), so while the target
/// question stands all three lands are still on the battlefield and the Agent
/// is still the 1/1 it was printed as. The two menus read the card's own words
/// back — "target creature" is a creature on either side of the table and
/// never a land, while "a land" is the activating seat's own and never the
/// opponent's — and the exactly-payable pool makes "the price came out of the
/// pool" an exact statement rather than an approximation.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn agent_of_shauku_sacrifices_a_land_to_pump_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), agent_of_shauku()])
        .battlefield(1, &[forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let agent = on_battlefield(&engine, p0, agent_of_shauku()).expect("the Agent is out");
    let mine = on_battlefield(&engine, p0, swamp()).expect("a Swamp is out");
    let theirs_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, agent), (1, 1), "a printed 1/1");
    assert_eq!(lands_of(&engine, p0).len(), 3, "three Swamps were dealt");

    // Two Swamps pay the {1}{B} and the third stays untapped: it is the land
    // the cost is about to name, so the pool holds exactly the price and
    // nothing else.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps pay {{1}}{{B}} and the third stays up for the sacrifice"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(agent, 0)),
        "the mana is in the pool, so the one line the Agent prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, agent_of_shauku(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat chooses the target");
    assert!(
        options.contains(&agent) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&mine) && !options.contains(&theirs_land),
        "a land is no creature: {options:?}"
    );
    // CR 601.2c before 601.2h: while this question stands, no cost has been
    // paid — the pump has not happened and every land is still there.
    assert_eq!(
        pt(&engine, agent),
        (1, 1),
        "the pump has not happened yet: the cost is paid after the target"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "and no land has been sacrificed yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![agent],
            },
        )
        .expect("the Agent was one of the creatures it offered");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which land to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat paying the cost is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&mine),
        "a land this seat controls is the whole of the answer: {options:?}"
    );
    assert!(
        !options.contains(&theirs_land),
        "`CR 701.21a`: a seat sacrifices only what it controls: {options:?}"
    );
    assert!(
        !options.contains(&agent),
        "the Agent is a creature and no land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the land the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, agent),
        (3, 1),
        "+2/+0 until end of turn on the creature it named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the creature it did not"
    );
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "the sacrificed land went to its owner's graveyard"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "and only the land that was named left the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{B}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, agent),
        "the Agent paid no tap: its whole price is mana and a land"
    );
}
