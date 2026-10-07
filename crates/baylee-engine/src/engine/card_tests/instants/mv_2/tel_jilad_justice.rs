//! `cards/instants/mv_2/tel_jilad_justice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tel-Jilad Justice — {1}{G} Instant: "Destroy target artifact. Scry 2."
///
/// Both printed sentences are the engine's answer rather than the card's, so
/// both are played on one board: the {1}{G} comes out of a pool only the three
/// Forests filled, the target question names the artifact across the table and
/// must decline the Elf standing beside it, and the scry is read as a *move*
/// of the top two cards rather than as a question that was asked. The destroy
/// is asserted while the scry question is already open, which is the one moment
/// that tells the two effects apart: the artifact is in its owner's graveyard
/// before the looking starts, so the scry cannot be what removed it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn tel_jilad_justice_destroys_an_artifact_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[tel_jilad_justice()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The two cards the scry is about to look at, named before anything is
    // cast: the list's last entry is the top of the library and its first is
    // the bottom, which is the order `Effect::Scry` reads the top `n` in.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped for three green, and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, tel_jilad_justice());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell names the target");
    assert_eq!((min, max), (1, 1), "one artifact, and the spell asks once");
    assert!(
        options.contains(&ring),
        "the artifact across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is not an artifact, and \"target artifact\" is read rather \
         than skipped: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the destroy is an effect and not a cost: the artifact is still on the \
         battlefield while the target is being named"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] },)
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered is the one that dies");

    // The spell's two effects resolve in the order they are printed, so the
    // destroy has already happened when the scry's question arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact is in its owner's graveyard before the looking starts"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and has left the battlefield, which is what destroying it means"
    );

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
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("one of the two cards just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top card"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "scry reorders and draws nothing"
    );
    assert!(
        in_graveyard(&engine, p0, tel_jilad_justice()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing else on the table moved: the spell destroyed one artifact \
         and looked at two cards"
    );
}
