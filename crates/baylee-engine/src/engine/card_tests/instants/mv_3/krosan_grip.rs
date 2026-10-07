//! `cards/instants/mv_3/krosan_grip.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Krosan Grip: "Destroy target artifact or enchantment." The test casts
/// Krosan Grip targeting the opponent's Sol Ring (`quiet_artifact`), confirms
/// creature permanents are not valid targets, and asserts the artifact is
/// destroyed. Split second is
/// `krosan_grip_s_split_second_leaves_only_mana_abilities`.
#[test]
fn krosan_grip_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(43, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[krosan_grip()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("opponent controls Sol Ring");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls an Elf");

    cast_from_hand(&mut engine, p0, krosan_grip());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ring),
        "target artifact or enchantment — Sol Ring qualifies"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("Sol Ring is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted artifact is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the destroyed artifact is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, krosan_grip()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}

/// Krosan Grip's split second (CR 702.61a): "As long as this spell is on the
/// stack, players can't cast other spells or activate abilities that aren't
/// mana abilities."
///
/// With the Grip on the stack the opponent is offered neither the Bolt in
/// hand nor Lotleth Troll's regeneration, which is how Grip is played: the
/// artifact's controller gets no answer. The Elves' mana ability stays
/// (CR 702.61b), and a Bolt cast anyway is refused. Once the Grip has
/// resolved the lock is gone and the Bolt is offered again.
#[test]
fn krosan_grip_s_split_second_leaves_only_mana_abilities() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(44, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[krosan_grip()])
        .battlefield(
            1,
            &[
                quiet_artifact(),
                llanowar_elves(),
                lotleth_troll(),
                mountain(),
                swamp(),
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("opponent controls Sol Ring");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls an Elf");
    let troll = on_battlefield(&engine, p1, lotleth_troll()).expect("and a Lotleth Troll");
    let bolt = in_hand(&engine, p1, lightning_bolt()).expect("and holds a Bolt");
    let mountain_id = on_battlefield(&engine, p1, mountain()).expect("and a Mountain");

    cast_from_hand(&mut engine, p0, krosan_grip());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("Sol Ring is a legal target");
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("the caster passes with the Grip on the stack");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "expected the opponent's priority, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the opponent holds priority over the Grip");
    assert!(
        legal
            .abilities
            .iter()
            .any(|(source, _)| *source == mountain_id),
        "the Mountain's mana ability is offered under split second (CR 702.61b)"
    );
    // A spell is offered as castable once its mana is floating, so the
    // opponent makes {R} first: without that, the Bolt would be missing from
    // the offer with or without split second.
    tap_mana_where(&mut engine, p1, |id| id == mountain_id);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected the opponent's priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.castable.contains(&bolt),
        "no spell may be cast while a split-second spell is on the stack"
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == troll),
        "nor a non-mana ability activated: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == elf),
        "a mana ability still may be (CR 702.61b): {:?}",
        legal.abilities
    );
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: bolt })
            .is_err(),
        "and a Bolt cast anyway is refused"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the Grip resolved and destroyed its target"
    );
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("the active player passes on an empty stack");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "expected the opponent's priority, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1);
    assert!(
        legal.castable.contains(&bolt),
        "with the Grip gone the Bolt is an instant again, off the {{R}} still floating"
    );
}
