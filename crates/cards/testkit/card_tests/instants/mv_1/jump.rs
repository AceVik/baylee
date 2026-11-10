//! `cards/instants/mv_1/jump.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jump costs {U} and prints a line: "Target creature gains flying until
/// end of turn." The test plays both halves. The offer is the one proof
/// you cannot read from the card: the card says "target creature" and not
/// "target creature you control", so two of your own Elves *and* the one
/// on the battlefield are in the same selection, and only the named one
/// gets flying. The turn change checks the duration: the same Elf is
/// grounded again in the opponent's main phase.
#[test]
fn jump_grants_flying_to_the_creature_it_targets_and_only_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[jump()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays grounded");
    let (target, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FLYING),
        "nothing has been granted anything yet"
    );

    // Mana before the assertion: the Island is tapped before the spell is
    // considered playable. The Elves are exempt because they are the
    // creatures this is about in a moment.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one Island, one blue, and neither Elf of mine paid in"
    );
    cast_with_floating(&mut engine, p0, jump());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&target) && options.contains(&bystander),
        "both creatures under my control are creatures: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"target creature you control\": {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the creature the question offered is the one that gains flying");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, target).contains(KeywordSet::FLYING),
        "the Elf the instant named is flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the Elf nobody named is still grounded"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the Elf across the table was a legal target and was not the target"
    );

    // "until end of turn": the next turn reads the same Elf grounded.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FLYING),
        "the grant lasts until end of turn and not a step longer"
    );
}
