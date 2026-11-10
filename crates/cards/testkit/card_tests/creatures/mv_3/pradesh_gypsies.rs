//! `cards/creatures/mv_3/pradesh_gypsies.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pradesh Gypsies cost `{2}{G}` for a 1/1 body and print one line: `{1}{G},
/// {T}: Target creature gets -2/-0 until end of turn`. Every number in that
/// sentence is read off one board, so the first three are paid on turn one out
/// of three Forests and the last two on turn two out of the same three, once
/// the creature is past summoning sickness (CR 302.6) — target named at
/// CR 601.2c, the mana and the `{T}` only at CR 601.2h. The pump is aimed at a
/// creature whose power and toughness have been pulled apart by Giant Growth:
/// `2/4` is the only body that reads `-2` with a toughness line of `+0`, where
/// `-2/-2` would read `2/2` and a pump that never happened `4/4`.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn pradesh_gypsies_taps_two_mana_for_minus_two_power_and_leaves_toughness_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        // The Elf across the table: "target creature" is not "a creature you
        // control", so the offer below has to name it and the pump must leave
        // it alone.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[pradesh_gypsies(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("and one across the table");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before anything");

    // Three Forests, and the Elf kept back: it is the creature both spells are
    // about, and `tap_all_mana` presses every ability whose whole price is its
    // own {T} — a mana creature's included (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and nothing off the Elf"
    );

    cast_with_floating(&mut engine, p0, pradesh_gypsies());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let gypsies = on_battlefield(&engine, p0, pradesh_gypsies()).expect("the Gypsies resolved");
    assert_eq!(pt(&engine, gypsies), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{G}} is three, and three Forests paid for it"
    );

    // A creature's {{T}} ability waits for its controller's next turn
    // (CR 302.6), which is why the pump is played a turn later and out of the
    // same three sources the cast came from.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gypsies),
        "the untap step ran: the Gypsies came back with the Forests"
    );
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the same three Forests, tapped a second turn"
    );

    // Giant Growth first, so the target's two halves disagree and the pump's
    // "-2 and +0" is a body no single number of the card could fake.
    cast_with_floating(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf) && options.contains(&theirs),
        "Giant Growth reaches either side of the table: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, elf), (4, 4), "the printed 1/1 is a 4/4");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}} is spent and the Gypsies' {{1}}{{G}} is still floating"
    );

    // The card's own line, read where the engine offers it: `activate` looks
    // the ability up in `LegalActions`, so an unpayable or missing line would
    // panic here rather than pass quietly.
    activate(&mut engine, p0, pradesh_gypsies(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Gypsies' line targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert!(
        options.contains(&elf) && options.contains(&theirs) && options.contains(&gypsies),
        "\"target creature\" is any creature — the Elf just pumped, the Elf \
         across the table and the Gypsies themselves: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays, so neither half of the
    // price has moved while the question stands.
    assert!(
        !is_tapped(&engine, gypsies),
        "{{T}} is the last step of the activation"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the mana is with it: nothing is spent while the target is unanswered"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the pumped Elf was one of the options");
    assert!(
        is_tapped(&engine, gypsies),
        "{{T}} is paid once the target is named"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} went with it, so the three Forests paid for the Growth and \
         the pump and nothing is left"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (2, 4),
        "-2/-0 on the 4/4: power down two, toughness exactly where it was"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the Elf the ability did not name never moved"
    );
}
