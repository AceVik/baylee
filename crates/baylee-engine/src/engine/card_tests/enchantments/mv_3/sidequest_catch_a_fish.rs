//! `cards/enchantments/mv_3/sidequest_catch_a_fish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The upkeep looks even when a land is on top: acknowledging that private
/// information leaves the front face unchanged and adds no card to hand.
#[test]
fn sidequest_catch_a_fish_looks_at_an_ineligible_land_without_transforming() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(108, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[sidequest_catch_a_fish()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, sidequest_catch_a_fish());
    pass_until(&mut engine, stack_is_empty);

    let quest = on_battlefield(&engine, p0, sidequest_catch_a_fish())
        .expect("Sidequest: Catch a Fish on battlefield");
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest is on face 0"
    );

    let t = types(&engine, quest);
    assert!(
        t.contains(TypeSet::ENCHANTMENT),
        "Sidequest is an enchantment"
    );
    assert!(!t.contains(TypeSet::LAND), "Sidequest is not a land");

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == quest),
        "front face offers no activated abilities with floating mana"
    );

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PutIntoHand,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Main
    });

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "two turns pass, so p0 draws once for its own draw step (CR 504.1) — \
         the ineligible top land does not add another draw"
    );
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest remains on face 0 in the following turn"
    );
}

/// Cooking Campsite, the back face: "{3}, {T}, Sacrifice an artifact: Put a
/// +1/+1 counter on each creature you control. Activate only as a sorcery."
#[test]
fn cooking_campsite_sacrifices_an_artifact_for_a_counter_on_each_of_your_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                sidequest_catch_a_fish(),
                grizzly_bears(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    // Turn it over while the hands are still being kept, so the front face's
    // upkeep look never gets a step to ask its question.
    let campsite = on_battlefield(&e, p0, sidequest_catch_a_fish()).unwrap();
    let def = baylee_cards::by_index(sidequest_catch_a_fish()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(campsite, def, 1));
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bears = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let artifact = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    let their_elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();

    activate(&mut e, p0, sidequest_catch_a_fish(), 1);
    if let Pending::ChooseCards { options, .. } = e.pending().clone() {
        assert!(options.contains(&artifact));
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bears, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&e, elf, CounterKind::P1P1), 1);
    assert_eq!(
        counters_on(&e, their_elf, CounterKind::P1P1),
        0,
        "only creatures you control"
    );
    assert_eq!(e.state().object(artifact).unwrap().zone, Zone::Graveyard);
    assert!(is_tapped(&e, campsite));
}

/// Cooking Campsite: "{T}: Add {W}." One white, no more, and the tap is the
/// cost.
#[test]
fn cooking_campsite_taps_for_exactly_one_white() {
    let p0 = PlayerId::new(0);
    let (mut engine, campsite) =
        placed_face(sidequest_catch_a_fish(), 1, &[]).expect("the Campsite on the battlefield");
    assert!(!is_tapped(&engine, campsite));
    activate(&mut engine, p0, sidequest_catch_a_fish(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{T}}: Add {{W}}");
    assert_eq!(pool.total(), 1, "and nothing else");
    assert!(is_tapped(&engine, campsite), "{{T}} is part of the cost");
}

/// "Activate only as a sorcery": the same Campsite, artifact and {3} that
/// offer the ability in your own main phase do not offer it in the
/// opponent's.
#[test]
fn cooking_campsite_artifact_sacrifice_is_not_offered_in_the_opponents_main_phase() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[sidequest_catch_a_fish(), quiet_artifact()])
        .start();
    let campsite = on_battlefield(&e, p0, sidequest_catch_a_fish()).unwrap();
    let def = baylee_cards::by_index(sidequest_catch_a_fish()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(campsite, def, 1));
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let give_three = |e: &mut Engine<RegistryLookup>| {
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 3);
        e.refresh_offer();
    };
    let offered = |e: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = e.pending().clone() else {
            panic!("expected priority, got {:?}", e.pending())
        };
        legal.abilities.contains(&(campsite, 1))
    };

    give_three(&mut e);
    assert!(
        offered(&e),
        "control: in your own main phase, empty stack, with {{3}} and an artifact, it is offered"
    );

    // The opponent's main phase: p1 passes with an empty stack, and p0
    // holds priority with the same resources.
    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
        "p0 gets priority in the opponent's main phase, got {:?}",
        e.pending()
    );
    assert_eq!(e.state().turn.active, p1);
    give_three(&mut e);
    assert!(
        !offered(&e),
        "\"Activate only as a sorcery\": not offered in the opponent's main phase"
    );
}
