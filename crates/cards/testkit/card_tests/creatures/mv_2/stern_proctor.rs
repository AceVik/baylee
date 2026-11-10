//! `cards/creatures/mv_2/stern_proctor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stern Proctor — {U}{U} — 1/2 Human Wizard: "When this creature enters,
/// return target artifact or enchantment to its owner's hand."
///
/// The board holds one of each thing the filter names and one thing it does
/// not: my Sol Ring, a Luminarch Ascension across the table, and an Elf that
/// must stay off the menu. Bouncing the *opponent's* enchantment is what
/// proves all three words at once — the type is read (a creature is not
/// offered), the side of the table is not (their permanent is), and "to its
/// owner's hand" is where it lands, not the hand of the seat that aimed it.
#[test]
fn stern_proctor_bounces_an_artifact_or_enchantment_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), quiet_artifact()])
        .hand(0, &[stern_proctor()])
        .battlefield(1, &[luminarch_ascension(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {U}{U} off the two Islands, and the Sol Ring named as the source kept
    // back: it taps for {C}{C} of its own, which pays none of a double-blue
    // cost, and it is the artifact this trigger is about to be offered.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    tap_mana_except(&mut engine, p0, ring);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, two blue, and the Sol Ring untouched"
    );
    cast_with_floating(&mut engine, p0, stern_proctor());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Proctor's controller chooses its target");

    let ascension =
        on_battlefield(&engine, p1, luminarch_ascension()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert!(
        options.contains(&ring) && options.contains(&ascension),
        "\"target artifact or enchantment\" is either type and either side of \
         the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither type, and the Elf across the table is the \
         counter-half that says the filter is read: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ascension],
            },
        )
        .expect("the permanent the question offered is the one that moves");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, luminarch_ascension()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, luminarch_ascension()).is_some(),
        "\"to its owner's hand\" — the seat that owns it, not the seat that \
         aimed the bounce"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the trigger did not name never moved"
    );
    assert_eq!(
        pt(
            &engine,
            on_battlefield(&engine, p0, stern_proctor()).expect("the Proctor resolved")
        ),
        (1, 2),
        "and the body that did it is the 1/2 the card prints"
    );
}
