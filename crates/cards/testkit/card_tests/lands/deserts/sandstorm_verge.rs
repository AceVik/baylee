//! `cards/lands/deserts/sandstorm_verge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sandstorm Verge: "{T}: Add {C}." / "{3}, {T}: Target creature can't block this turn. Activate only as a sorcery."
///
/// The first line, on its own: the Desert taps for colourless mana. The
/// second one is the test below this.
#[test]
fn sandstorm_verge_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, forest())
        .battlefield(0, &[sandstorm_verge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let verge = on_battlefield(&engine, p0, sandstorm_verge()).expect("Verge deployed");
    activate(&mut engine, p0, sandstorm_verge(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, verge));
}

/// Sandstorm Verge's second printed line: "{3}, {T}: Target creature can't
/// block this turn. Activate only as a sorcery."
///
/// The grant is a pump with no numbers — `KeywordSet::CANT_BLOCK` until end
/// of turn — so it is read twice on purpose. The keyword is on the creature
/// the ability named, which says the effect happened; and that creature is
/// gone from the blocks the engine offers, which says a rule reads it.
/// Either half alone would be green on a bit nothing consults.
#[test]
fn sandstorm_verge_takes_one_creature_out_of_the_defence() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(108, forest())
        .battlefield(
            0,
            &[
                sandstorm_verge(),
                forest(),
                forest(),
                forest(),
                rootbreaker_wurm(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let verge = on_battlefield(&engine, p0, sandstorm_verge()).expect("Verge deployed");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the 6/6 is seated");
    let guards = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(guards.len(), 2, "two creatures, and the ability names one");

    // The three Forests and nothing else: the Verge pays {T} out of its own
    // cost, and the Wurm has to stay untapped to attack with.
    tap_mana_where(&mut engine, p0, |id| id != verge);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "exactly the {{3}} the printed price asks for"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(verge, 1)),
        "at your own main with {{3}} floating and the land untapped, the \
         second line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sandstorm_verge(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the ability asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&guards[0]) && options.contains(&wurm),
        "\"target creature\" is any creature on the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guards[0]],
            },
        )
        .expect("the creature came out of the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, verge), "the {{T}} in the cost was paid");
    assert!(
        keywords(&engine, guards[0]).contains(KeywordSet::CANT_BLOCK),
        "the creature the ability named carries the restriction"
    );
    assert!(
        !keywords(&engine, guards[1]).contains(KeywordSet::CANT_BLOCK),
        "and the one beside it does not"
    );

    let blocks = attack_and_collect_blocks(&mut engine, wurm, p1);
    assert_eq!(
        blocks.iter().map(|o| o.blocker).collect::<Vec<_>>(),
        vec![guards[1]],
        "one Elf may block the Wurm and the other may not: {blocks:?}"
    );
}
