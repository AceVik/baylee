//! `cards/artifacts/mv_3/scrapheap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scrapheap — {3} artifact: "Whenever an artifact or enchantment is put
/// into your graveyard from the battlefield, you gain 1 life."
///
/// Three Krosan Grips in one main phase read every word of that sentence off
/// one board: the first destroys this seat's own Sol Ring (an *artifact* of
/// mine dying), the second its Exploration (the *enchantment* half of the
/// disjunction), and the third the Sol Ring across the table — an artifact
/// dying where `ControlledByYou` has to decline it and the life total has to
/// stay where it was. Six tapped Forests pay for all three, so an emptied
/// pool says the Grips were paid for rather than merely announced, and the
/// Scrapheap itself never dies, so every point of life belongs to the printed
/// sentence and not to a look-back on its own death (CR 603.6c).
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn scrapheap_gains_one_life_for_your_artifacts_and_enchantments_and_ignores_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                scrapheap(),
                quiet_artifact(),
                exploration(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[krosan_grip(), krosan_grip(), krosan_grip()])
        .battlefield(1, &[quiet_artifact()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let chant = on_battlefield(&engine, p0, exploration()).expect("my Exploration is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(engine.state().players[0].life, 20, "nothing has died yet");

    // Nine Forests — three Grips at {2}{G} each, and the third cast is the
    // one this test is really about — with the Sol Ring named as the
    // printing kept back: it is the permanent the first Grip is about, and a
    // source tapped for mana is a source whose status has already changed
    // for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Forests tapped for nine green, and the Sol Ring still standing"
    );

    // (1) An artifact of my own is put into my graveyard.
    cast_with_floating(&mut engine, p0, krosan_grip());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast the Grip aims it");
    assert!(
        options.contains(&ring) && options.contains(&chant) && options.contains(&theirs),
        "\"target artifact or enchantment\" reaches every one of them, on \
         either side of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the artifact \
         it names is still on the battlefield"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("my own Sol Ring was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the Grip named is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"Whenever an artifact … is put into your graveyard from the \
         battlefield, you gain 1 life\": one artifact of mine, one life"
    );

    // (2) The other half of the disjunction: an enchantment of my own.
    cast_with_floating(&mut engine, p0, krosan_grip());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&chant),
        "the Exploration is an enchantment this seat controls: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chant],
            },
        )
        .expect("the Exploration was still an enchantment I control");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, exploration()).is_some(),
        "the enchantment the Grip named is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "an enchantment counts as much as an artifact: a filter that read only \
         the first disjunct would have left this at 21"
    );

    // (3) An artifact dies and it is not mine, which is where the word
    // `ControlledByYou` is read.
    cast_with_floating(&mut engine, p0, krosan_grip());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Sol Ring across the table was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its own owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "an artifact dying across the table is no artifact of mine: the \
         Scrapheap gained nothing for it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the seat whose artifact died gains nothing either — the ability \
         belongs to the Scrapheap's controller"
    );
    assert!(
        on_battlefield(&engine, p0, scrapheap()).is_some(),
        "the Scrapheap outlived all three, so nothing here measured its own \
         death instead of the sentence it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three {{1}}{{G}} out of six green: the Grips were paid for, not \
         merely announced"
    );
}
