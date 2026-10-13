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

/// "Enchant creature you control": an opposing creature on the table is not
/// among the targets offered when the Aura is cast.
#[test]
fn journey_to_eternity_offers_only_creatures_its_caster_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[forest(), swamp(), plains(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[journey_to_eternity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the caster's Elves");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the opponent's Bears");
    cast_from_hand(&mut engine, p0, journey_to_eternity());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the Aura's target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf), "your own creature is enchantable");
    assert!(
        !options.contains(&bears),
        "\"Enchant creature you control\": an opponent's creature is not offered"
    );
}

/// Atzal's "{3}{B}{G}, {T}: Return target creature card from your graveyard
/// to the battlefield": your graveyard only, the cost taps Atzal and spends
/// the whole {3}{B}{G}.
#[test]
fn atzal_returns_a_creature_card_from_your_own_graveyard_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[journey_to_eternity(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    // Turned over while the hands are still being kept: it is the back face
    // under test, not the way it arrives.
    let atzal = on_battlefield(&engine, p0, journey_to_eternity()).unwrap();
    let def = baylee_cards::by_index(journey_to_eternity()).unwrap();
    assert!(engine.dev_state_mut(p0).unwrap().transform(atzal, def, 1));
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    bury(&mut engine, &[mine, theirs]);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());

    let pool = &mut engine.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::Black, 1);
    pool.add(ManaColor::Green, 1);
    pool.add(ManaColor::Colorless, 3);
    engine.refresh_offer();

    activate(&mut engine, p0, journey_to_eternity(), 1);
    let options = aim_at(&mut engine, p0, mine);
    assert!(
        options.contains(&mine),
        "your own graveyard's creature card is a target"
    );
    assert!(
        !options.contains(&theirs),
        "\"from your graveyard\": the opponent's creature card is not a target"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the chosen creature card came back under your control"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "the opponent's creature card stayed where it was"
    );
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "and is still in their graveyard"
    );
    assert!(is_tapped(&engine, atzal), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{B}}{{G}} was paid in full"
    );
}
