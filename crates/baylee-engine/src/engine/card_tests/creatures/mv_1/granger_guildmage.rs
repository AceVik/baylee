//! `cards/creatures/mv_1/granger_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Granger Guildmage — {G}, 1/1 Human Wizard — prints two activated abilities
/// that are both priced at the same {T}: "{R}, {T}: This creature deals 1
/// damage to any target and 1 damage to you", and "{W}, {T}: Target creature
/// gains first strike until end of turn."
///
/// Two copies stand on the board because the two lines share one tap symbol:
/// the second is only reachable through a creature the first has not already
/// spent, and the copy that paid stands tapped and offers nothing while the
/// {W} for the other line is still floating — which is what tells a tap cost
/// from a price the board simply cannot pay. The ping is the sentence that has
/// to be played: "any target" raises one question carrying a player list
/// beside the objects, and the damage that follows lands on two different
/// seats, so a resolution that read both halves as one target would move a
/// single life total. The Elf across the table is the control — a filter that
/// prints no controller offers it, and it is not the creature the first strike
/// lands on.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn granger_guildmage_pings_any_target_then_arms_the_other_copy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                granger_guildmage(),
                granger_guildmage(),
                mountain(),
                plains(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mages = all_on_battlefield(&engine, p0, granger_guildmage());
    assert_eq!(
        mages.len(),
        2,
        "two copies, so the shared {{T}} is no limit"
    );
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mages[0]), (1, 1), "a printed 1/1");

    // The offer is read off the pool (`can_afford`), so the mana goes in first:
    // one Mountain and one Plains between them pay both printed lines.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mages[0], 0)) && legal.abilities.contains(&(mages[1], 1)),
        "both lines are affordable and both Mages are untapped: {:?}",
        legal.abilities
    );

    // ------------------------------------------- {R}, {T}: any target
    activate(&mut engine, p0, granger_guildmage(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is one choice over objects and players, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&mages[0]) && options.contains(&theirs),
        "\"any target\" reaches every creature on the table, on both sides of \
         it: {options:?}"
    );
    assert!(
        player_options.contains(&p1),
        "and a player is one of the things it reaches (CR 115.4): {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player the question enumerated is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "and \"1 damage to you\" is the seat that activated, not the one it \
         aimed at: one target, two life totals"
    );
    // CR 601.2h: the {R} and the tap were paid after the target was named, and
    // the {W} the second line needs is still in the pool (CR 500.5).
    assert_eq!(
        (
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::White),
            engine.state().players[0].mana_pool.total()
        ),
        (1, 1),
        "the {{R}} was the price and the {{W}} is still floating"
    );

    // The tap is a cost and not a label: the copy that paid is down, and the
    // other one is the only thing left to press.
    let paid = mages
        .iter()
        .copied()
        .find(|id| is_tapped(&engine, *id))
        .expect("the copy that pinged is tapped");
    let free = mages
        .iter()
        .copied()
        .find(|id| !is_tapped(&engine, *id))
        .expect("the copy that did not ping is still standing");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(free, 1)),
        "the {{W}} is in the pool and an untapped Mage may spend it: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == paid),
        "a tapped Mage offers neither line — and the white one is the reading \
         that matters, because its mana is already floating: {:?}",
        legal.abilities
    );

    // ------------------------------ {W}, {T}: target creature gains
    // first strike
    activate(&mut engine, p0, granger_guildmage(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&paid) && options.contains(&theirs),
        "\"target creature\" carries no controller with it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        keywords(&engine, paid).contains(KeywordSet::FIRST_STRIKE),
        "the creature that was named gains first strike"
    );
    assert!(
        !keywords(&engine, free).contains(KeywordSet::FIRST_STRIKE),
        "the Mage that handed it out is not the Mage that got it"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and nothing crossed the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} was the second line's price"
    );
}
