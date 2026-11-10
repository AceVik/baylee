//! `cards/instants/mv_3/void_rend.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Void Rend prints two sentences: "This spell can't be countered" and
/// "Destroy target nonland permanent". The destroy half is played on a board
/// carrying every word of the filter — a creature and an artifact across the
/// table are the nonland permanents, and the Forest beside them plus the three
/// lands paying for the spell are what "nonland" has to decline — so the menu
/// reads exactly those two and the named creature is in its owner's graveyard
/// once the spell has resolved.
///
/// The other sentence is about the spell and not about any permanent, so it is
/// read off the object the engine puts on the stack: that object and that key
/// are what a counter effect would have to read before it could do anything.
#[test]
fn void_rend_destroys_a_nonland_permanent_and_carries_its_uncounterable_clause() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[plains(), island(), swamp()])
        .hand(0, &[void_rend()])
        .battlefield(1, &[forest(), llanowar_elves(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let my_land = on_battlefield(&engine, p0, island()).expect("my Island is out");

    // {W}{U}{B} out of the Plains, the Island and the Swamp: `cast_from_hand`
    // taps the three and pays for it, and the spell stops on its target
    // question on the way in (CR 601.2c).
    cast_from_hand(&mut engine, p0, void_rend());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target nonland permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell names the target");
    assert_eq!((min, max), (1, 1), "one permanent, and the spell asks once");
    assert!(
        options.contains(&victim) && options.contains(&rock),
        "\"target nonland permanent\" reaches a creature and an artifact alike: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and it reaches nothing else on this board — neither the Forest across \
         the table nor the three lands paying for the spell: {options:?}"
    );
    assert!(
        !options.contains(&their_land) && !options.contains(&my_land),
        "\"nonland\" is the word under test: a land is a permanent and never a \
         legal target for this spell: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    // The target is named, so the spell is on the stack now — and nothing has
    // happened to the board yet.
    let spell = on_stack(&engine, void_rend()).expect("the spell is waiting on the stack");
    assert!(
        keywords(&engine, spell).contains(KeywordSet::UNCOUNTERABLE),
        "\"This spell can't be countered\" rides the object the engine put on \
         the stack, which is where a counter would have to read it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the permanent it names is untouched while the spell is merely cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"Destroy target nonland permanent\": the named creature left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it is destroyed rather than merely moved somewhere else"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the nonland permanent the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and no land was touched, since none of them was ever on the menu"
    );
    assert!(
        in_graveyard(&engine, p0, void_rend()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );
}
