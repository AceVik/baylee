//! `cards/creatures/mv_5/ursapine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "fcf92abb-6c06-42d2-a3d1-f44ec997d56d"

/// Ursapine prints one line — `{G}: Target creature gets +1/+1 until end of
/// turn` — and it is a 3/3 on top of that, so the card is a price, a target and
/// a body. The board carries three creatures on purpose: an Elf of mine, the
/// Elf across the table and the Ursapine itself, which is what makes
/// `Filter::CREATURE` a claim about *any* creature rather than "creatures you
/// control" — the offer has to name all three. The pump then has to land on
/// exactly the one that was named, and the price has to leave a mark: the `{G}`
/// out of a pool six tapped Forests filled (the cast's `{3}{G}{G}` took five of
/// it), with the Ursapine still standing because its price is mana and not the
/// tap symbol.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn ursapine_pumps_the_creature_it_names_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[ursapine()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // Six Forests and only those: the Elf is named as the printing kept back,
    // because it is the creature this test is about to aim the pump at and a
    // mana creature tapped for the cost would put its own {{G}} in the pool.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped, six green, and the untapped Elf gave nothing"
    );
    cast_with_floating(&mut engine, p0, ursapine());
    pass_until(&mut engine, stack_is_empty);
    let beast = on_battlefield(&engine, p0, ursapine()).expect("the Ursapine resolved");
    assert_eq!(pt(&engine, beast), (3, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the cast's {{3}}{{G}}{{G}} is spent and exactly the {{G}} the ability \
         charges is left floating"
    );
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the pump");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — so the claim is made with the mana
    // already there.
    activate(&mut engine, p0, ursapine(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the ability asks once"
    );
    assert!(
        options.contains(&host) && options.contains(&theirs) && options.contains(&beast),
        "\"target creature\" is any creature on either side of the table, the \
         Ursapine itself included: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "and the three standing creatures are the whole menu: {options:?}"
    );
    // CR 601.2c before CR 601.2h: the target is named while the mana is still
    // floating and nothing about the source has moved.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        !is_tapped(&engine, beast),
        "and the price is {{G}}, so nothing has tapped the Ursapine either"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} it charges came out of the pool"
    );
    assert!(
        !is_tapped(&engine, beast),
        "an activation that costs mana and no tap leaves the source standing"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the ability named, and not a toughness the card \
         never prints"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert_eq!(
        pt(&engine, beast),
        (3, 3),
        "nor its own source, which the offer named and the answer did not"
    );

    // "until end of turn": a turn later the Elf is a printed 1/1 again, so the
    // +1/+1 was a duration and not a counter the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
