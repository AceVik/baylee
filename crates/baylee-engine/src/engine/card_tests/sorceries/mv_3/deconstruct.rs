//! `cards/sorceries/mv_3/deconstruct.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deconstruct — {2}{G} sorcery: "Destroy target artifact. Add {G}{G}{G}."
///
/// Both halves are one scenario because the second is what makes the first
/// worth paying for. Three Forests are tapped to cover the {2}{G}, and the
/// pool they filled is empty again the moment the spell's cost is paid at
/// CR 601.2h — so the three green standing in it after the spell resolves can
/// only be the card's own effect, and "Add {G}{G}{G}" is read as a number
/// rather than as a shuffle of whatever was already floating.
///
/// The artifact destroyed is the *opponent's* Sol Ring with an Elf beside it,
/// which is the other half of the target filter: "target artifact" reaches
/// across the table and still declines a creature. The Sol Ring is read out
/// of its owner's graveyard and not merely gone from the battlefield, and the
/// Elf that was never named never moves.
#[test]
fn deconstruct_destroys_the_artifact_it_targets_and_leaves_three_green_behind() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[deconstruct()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // pool and not the untapped lands: three Forests into the pool first, so
    // the claim below is about the card and not about the board behind it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three green, and nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, deconstruct()).expect("the sorcery is in hand");
    assert!(
        legal.castable.contains(&spell),
        "an artifact stands across the table, so the sorcery has the target it \
         needs: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, deconstruct());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&rock),
        "\"target artifact\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "an Elf is a creature and no artifact: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while the question stands the mana is still
    // floating and the Sol Ring is still on the battlefield.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the artifact is still standing while the target is chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered is the one that dies");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{G}} came out of the pool the moment the cost was paid"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying an artifact is no mana ability, so the sorcery is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard, not the caster's"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        3,
        "\"Add {{G}}{{G}}{{G}}\" — three green off the resolving spell"
    );
    assert_eq!(
        pool.total(),
        3,
        "and nothing else in the pool: the {{2}}{{G}} was spent and the effect \
         put exactly its own three back"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
}
