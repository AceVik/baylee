//! `cards/instants/mv_2/magma_jet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn magma_jet_kills_a_one_one_across_the_table_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[magma_jet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, elf), (1, 1), "a 1/1 for two damage to kill");

    // The two cards the scry is about to look at, named before anything is
    // cast: the list's last entry is the top of the library and its first is
    // the bottom, which is the order `Effect::Scry` reads the top `n` in.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red: nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, magma_jet()).expect("the Jet is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{1}}{{R}} is in the pool, so the Jet is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, magma_jet());
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
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&elf),
        "the Elf across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    // The spell resolves and the scry is the last thing it does, so this is
    // where the card asks its second question — the walk stops on the
    // question rather than answering it, because the *answer* is the subject.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("one of the two just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and what died left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the \
         player whose board it stood on"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the two red Mountains"
    );

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(library.len(), library_before.len(), "scry draws nothing");
}
