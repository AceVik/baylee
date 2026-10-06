//! `cards/sorceries/mv_3/ice_storm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ice Storm is `{2}{G}` for one printed sentence: "Destroy target land."
///
/// Two words in that sentence only a board can read. "Target land" is a
/// filter, so the menu is the proof — both seats' Forests are on it, at four
/// lands exactly, while the Sol Ring and the Elf across the table are not.
/// "Destroy" is a move, so the land the spell named has to be in its owner's
/// graveyard afterwards with every permanent it did not name still standing.
/// The offer is claimed with the three Forests already tapped, because
/// `legal.castable` is filtered through `can_afford` and that reads the pool
/// rather than the untapped lands.
#[test]
#[allow(clippy::too_many_lines)]
fn ice_storm_destroys_the_land_it_names_and_no_other_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[forest(), quiet_artifact(), llanowar_elves()])
        .hand(0, &[ice_storm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let mine = on_battlefield(&engine, p0, forest()).expect("my Forest is out");

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // mana pool rather than the untapped lands: with nothing floating the
    // {2}{G} is unpayable and the Storm is not offered at all.
    let card = in_hand(&engine, p0, ice_storm()).expect("the Storm is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("p0 holds a quiet main phase, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{G}}, so the Storm is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped, three green: nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with {{2}}{{G}} floating the one sentence the card prints is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, ice_storm());
    // CR 601.2c: the target is named before anything is destroyed, so the land
    // that is about to die is still standing while this question is open.
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster is the seat that aims it");
    assert_eq!((min, max), (1, 1), "one land, and the spell asks once");
    assert!(
        options.contains(&mine),
        "a land this seat controls is a land — tapped, and still one: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the four lands on the battlefield — three of mine and one of theirs — \
         and no other permanent: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "an artifact is no land, even one with a land's job: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is no land: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "nothing has been destroyed while the target question is still open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("their Forest was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"Destroy target land\": the land the spell named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and it has left the battlefield, which is the destruction itself"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "the land the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and neither did the permanent that is no land"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor the creature standing beside it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{G}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, ice_storm()).is_some(),
        "and the sorcery itself resolved into its owner's graveyard"
    );
}
