//! `cards/creatures/artifacts/mv_4/roaming_throne.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Roaming Throne — {4} — Artifact Creature — Golem — 4/4, ward {2}, "As this
/// creature enters, choose a creature type", "This creature is the chosen type
/// in addition to its other types", and "If a triggered ability of another
/// creature you control of the chosen type triggers, it triggers an additional
/// time." The Throne is **cast** rather than seated, because only a real entry
/// asks the question, and "Ally" is named at it — a type the card is not printed
/// with, so the answer has to travel to the filter that reads it. The witness is
/// Umara Raptor, a Bird Ally whose rally counter the kit's own `a_two_two_raptor`
/// settles at exactly one (a 2/2), so a Raptor that enters beside the Throne and
/// comes off the stack as a 3/3 carrying *two* +1/+1 counters is that chosen type
/// matching a creature you control and that creature's trigger really firing an
/// additional time. (Ward {2} only answers a spell aimed at the Throne and is not
/// exercised here.)
#[test]
fn roaming_throne_names_a_type_and_doubles_the_rally_of_that_type() {
    let p0 = PlayerId::new(0);
    let ally = baylee_core::generated::subtypes::creature::ALLY;
    let golem = baylee_core::generated::subtypes::creature::GOLEM;

    // Four Forests pay the Throne's {4} and the three Islands are held back for
    // the Raptor's {2}{U}: both casts happen inside one main phase, so the pool
    // has to survive the first of them (CR 500.5).
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[roaming_throne(), umara_raptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 4, "four Forests are on the table");
    assert_eq!(
        tap_mana_where(&mut engine, p0, |id| forests.contains(&id)),
        4,
        "the four Forests and nothing else: the Islands are the Raptor's blue"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four green in the pool, with the Islands still standing"
    );

    cast_with_floating(&mut engine, p0, roaming_throne());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, p0, "the seat that cast it names the type");
    assert!(
        options.contains(&ally) && options.contains(&golem),
        "\"choose a creature type\" offers every creature type — the Golem it \
         prints and the Ally it does not: {} options",
        options.len()
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(ally))
        .expect("the Ally was one of the types it offered");

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, roaming_throne()).is_some()
    });
    let throne = on_battlefield(&engine, p0, roaming_throne()).expect("the Throne resolved");
    let kinds = types(&engine, throne);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "the artifact creature the card prints: {kinds:?}"
    );
    assert_eq!(pt(&engine, throne), (4, 4), "and its printed 4/4 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the four green paid the {{4}} to the last mana"
    );

    // The witness: `cast_from_hand` taps the three Islands for the {2}{U}, and
    // the Raptor's own rally — the counter it puts on itself as it enters — is
    // the triggered ability the Throne's replacement sentence is about.
    cast_from_hand(&mut engine, p0, umara_raptor());
    pass_until(&mut engine, stack_is_empty);
    let raptor = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor resolved");
    assert_eq!(
        counters_on(&engine, raptor, CounterKind::P1P1),
        2,
        "\"it triggers an additional time\": the rally counted its +1/+1 twice, \
         where the Raptor standing alone takes one"
    );
    assert_eq!(
        pt(&engine, raptor),
        (3, 3),
        "a printed 1/1 with the single rally counter is the 2/2 the kit's own \
         `a_two_two_raptor` documents; two counters is the Throne's extra trigger"
    );
}
