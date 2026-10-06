//! `cards/lands/utility/smoldering_spires.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smoldering Spires: "This land enters tapped." / "When this land enters, target creature can't block this turn." / "{T}: Add {R}."
///
/// This test used to assert the opposite — `stack_is_empty` right after the
/// land arrived, because the trigger had been left off the card for want of
/// a way to say "can't block". The pin is flipped rather than deleted: a
/// pinned limitation is a test whose breaking is the success, and what it
/// was watching for is exactly what now has to happen.
///
/// Playing the land is what fires it, so the test plays it. Three things
/// follow and each is its own sentence of the card: the land arrives
/// tapped, the trigger asks for one creature and the creature it names is
/// gone from the blocks the engine offers while its twin is still there,
/// and the land untaps a turn later to make its {R}. The mana ability sits
/// at index 1 now, which is printed order and what `Looming Spires` — the
/// same land with a different rider — has carried since codegen wrote it.
#[test]
fn smoldering_spires_enters_tapped_and_takes_one_creature_out_of_the_defence() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[rootbreaker_wurm()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[smoldering_spires()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the 6/6 is seated");
    let guards = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(guards.len(), 2, "two creatures, and the rider names one");

    let spires = play_land(&mut engine, p0, smoldering_spires());
    assert!(
        entered_tapped(&engine, spires),
        "Smoldering Spires enters tapped"
    );

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the enters trigger asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&guards[0]) && options.contains(&guards[1]) && options.contains(&wurm),
        "\"target creature\" is any creature on the table, the controller's \
         own included (CR 115.1): {options:?}"
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

    assert!(
        keywords(&engine, guards[0]).contains(KeywordSet::CANT_BLOCK),
        "the creature the trigger named carries the restriction"
    );
    assert!(
        !keywords(&engine, guards[1]).contains(KeywordSet::CANT_BLOCK),
        "and the one beside it does not, so what follows is about the target"
    );

    let blocks = attack_and_collect_blocks(&mut engine, wurm, p1);
    assert_eq!(
        blocks.iter().map(|o| o.blocker).collect::<Vec<_>>(),
        vec![guards[1]],
        "one Elf may block the Wurm and the other may not: {blocks:?}"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("blocking with nothing is always legal");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, spires), "untaps on next turn");

    activate(&mut engine, p0, smoldering_spires(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert!(is_tapped(&engine, spires));
}
