//! `cards/creatures/mv_5/joven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "9e767d44-feff-449a-bed9-0866c7bce846"

/// Joven — {3}{R}{R} legendary Human Rogue, 3/3, with "{R}{R}{R}, {T}: Destroy
/// target noncreature artifact." Both words of the printed filter need their own
/// bystander on the same menu, and neither can be assumed: the Sol Ring across
/// the table is an artifact and no creature, the Myr Retriever beside it is an
/// artifact *and* a creature, and the Elves are a creature and no artifact — so
/// a menu holding exactly the Sol Ring is what reads
/// `Filter::And(ARTIFACT, NONCREATURE)` rather than either word alone. Eight
/// Mountains pay the {3}{R}{R} and leave exactly the {R}{R}{R} the activation
/// charges, and the target question stands before the costs (CR 601.2c before
/// CR 601.2h), so Joven is still standing and the three red still floating
/// while the answer is being given.
#[test]
#[allow(clippy::too_many_lines)]
fn joven_destroys_a_noncreature_artifact_and_declines_an_artifact_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        // Joven starts on the battlefield: his ability's {T} can be paid only
        // by a creature its controller has held since the turn began
        // (CR 302.6), so one cast this turn could not activate it.
        .battlefield(0, &[mountain(), mountain(), mountain(), joven()])
        .battlefield(1, &[quiet_artifact(), myr_retriever(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Mountains are the whole price: the {R}{R}{R} his ability
    // charges, floating inside this one main phase (CR 500.5).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, three red"
    );

    let joven_id = on_battlefield(&engine, p0, joven()).expect("Joven is out");
    assert_eq!(pt(&engine, joven_id), (3, 3), "the body the card prints");
    assert!(!is_tapped(&engine, joven_id), "and he is untapped");

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring is out");
    let myr = on_battlefield(&engine, p1, myr_retriever()).expect("the Myr Retriever is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds a quiet main phase, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(joven_id, 0)),
        "the one line the card prints, now that its {{R}}{{R}}{{R}} is in the \
         pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, joven(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target noncreature artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one artifact, and the ability asks once"
    );
    assert!(
        options.contains(&ring),
        "the Sol Ring across the table is an artifact and no creature: {options:?}"
    );
    assert!(
        !options.contains(&myr),
        "the Myr Retriever is an artifact *and* a creature, so \"noncreature\" \
         declines it: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elves are a creature and no artifact at all: {options:?}"
    );
    assert!(
        !options.contains(&joven_id),
        "Joven is a creature and not a legal target for his own ability: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and those exclusions leave the Sol Ring as the whole menu: {options:?}"
    );

    // CR 601.2c before CR 601.2h: while the question stands, neither price is
    // paid — Joven is still standing and the three red are still in the pool.
    assert!(
        !is_tapped(&engine, joven_id),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{R}}{{R}}{{R}} is not spent yet either"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Sol Ring was the one option the question enumerated");
    assert!(
        is_tapped(&engine, joven_id),
        "{{T}} is paid by Joven himself, and he is no mana ability's source"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}}{{R}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact the ability named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, myr_retriever()).is_some(),
        "the artifact creature was never on the menu and never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor did the Elves"
    );
    assert!(
        on_battlefield(&engine, p0, joven()).is_some(),
        "the ability destroys its target and Joven outlives it"
    );
}
