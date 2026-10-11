//! `cards/enchantments/auras/mv_3/journey_to_eternity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn journey_to_eternity_returns_creature_and_transforms_into_atzal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(
            0,
            &[
                forest(),
                swamp(),
                plains(),
                llanowar_elves(),
                ashnods_altar(),
            ],
        )
        .hand(0, &[journey_to_eternity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    cast_from_hand(&mut engine, p0, journey_to_eternity());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "the controlled creature is a legal target"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, journey_to_eternity())
        .expect("Journey to Eternity on battlefield");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf),
        "Journey to Eternity is attached to the creature"
    );

    let altar =
        on_battlefield(&engine, p0, ashnods_altar()).expect("Ashnod's Altar on battlefield");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards { prompt, .. } = engine.pending().clone() else {
        panic!(
            "expected sacrifice choice for Ashnod's Altar, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !went_from_graveyard_to_exile(&engine, aura),
        "the Aura returns directly from its graveyard"
    );

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the enchanted creature returned to the battlefield under your control"
    );
    let atzal = on_battlefield(&engine, p0, journey_to_eternity())
        .expect("Atzal, Cave of Eternity returned transformed");
    assert_eq!(
        engine.state().object(atzal).map(|o| o.face_index),
        Some(1),
        "Atzal is on face 1"
    );

    assert!(!engine.state().journal.entries().iter().any(|entry| matches!(entry.event, crate::event::GameEvent::Transformed { object, .. } if object == aura)), "entering transformed is not a transform event");
    let t = types(&engine, atzal);
    assert!(t.contains(TypeSet::LAND), "Atzal is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Atzal is no longer an enchantment"
    );

    activate(&mut engine, p0, journey_to_eternity(), 0);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!(
            "expected color choice for Atzal mana ability, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "Atzal produced one black mana"
    );
}
