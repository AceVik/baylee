//! `cards/creatures/mv_1/grist_voracious_larva.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::partials_w1b_kit as kit;
use baylee_core::color::Color;
use baylee_core::generated::index;
use baylee_core::generated::subtypes::creature;

fn grist() -> CardIndex {
    card_index("196f8f13-d4df-4740-8746-8fed3bc71840")
}

#[track_caller]
fn back_at(loyalty: u16) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grist(), index::GRAY_OGRE])
        .hand(
            0,
            &[
                forest(),
                forest(),
                index::DARK_RITUAL,
                mountain(),
                serra_angel(),
                index::GRIZZLY_BEARS,
                lightning_bolt(),
            ],
        )
        .battlefield(
            1,
            &[
                index::JAYEMDAE_TOME,
                index::GLORIOUS_ANTHEM,
                index::GRAY_OGRE,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let larva = on_battlefield(&engine, kit::P0, grist()).expect("the Larva is placed");
    kit::turn_over(&mut engine, kit::P0, larva, grist());
    kit::set_loyalty(&mut engine, kit::P0, larva, loyalty);
    (engine, larva)
}

#[track_caller]
fn loyalty_ability(engine: &mut Engine<RegistryLookup>, grist: ObjectId, index: u32) {
    assert!(
        kit::offered(engine, grist).contains(&index),
        "loyalty ability {index} is offered"
    );
    kit::activate_object(engine, kit::P0, grist, index);
}

#[track_caller]
fn reanimate(engine: &mut Engine<RegistryLookup>, target: ObjectId, pay_green: Option<bool>) {
    tap_all_mana(engine, kit::P0);
    cast_with_floating(engine, kit::P0, index::REANIMATE);
    aim_at(engine, kit::P0, target);
    kit::settle(engine, kit::P0, |_, pending| match pending {
        Pending::YesNo {
            prompt: YesNoPrompt::PayMana { cost },
            ..
        } => {
            assert_eq!(
                *cost,
                baylee_core::mana!("{G}"),
                "the optional transform costs green"
            );
            Some(PlayerAction::YesNo(pay_green.unwrap_or(true)))
        }
        _ => None,
    });
}

#[test]
fn deathtouch_makes_one_combat_damage_lethal_to_a_two_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grist()])
        .battlefield(1, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let grist = on_battlefield(&engine, kit::P0, grist()).unwrap();
    let elves = on_battlefield(&engine, kit::P1, index::LLANOWAR_ELVES).unwrap();
    assert!(keywords(&engine, grist).contains(KeywordSet::DEATHTOUCH));

    kit::declare_attack(&mut engine, &[grist], kit::P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            kit::P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, grist)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(kit::zone_of(&engine, elves), Zone::Graveyard);
    assert!(kit::is_on_battlefield(&engine, grist));
    assert_eq!(kit::damage_on(&engine, grist), 1);
}

#[test]
fn an_ordinary_hand_cast_enters_without_the_transform_trigger() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[grist()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    tap_all_mana(&mut engine, kit::P0);

    cast_with_floating(&mut engine, kit::P0, grist());
    kit::settle(&mut engine, kit::P0, |_, pending| match pending {
        Pending::YesNo { .. } => panic!("a hand cast must not offer a transform: {pending:?}"),
        _ => None,
    });

    let grist = on_battlefield(&engine, kit::P0, grist()).unwrap();
    assert_eq!(kit::face_of(&engine, grist), 0);
    assert!(types(&engine, grist).contains(TypeSet::CREATURE));
}

#[test]
fn its_own_real_graveyard_return_can_transform_it_or_be_declined() {
    for pay in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[swamp(), forest()])
            .hand(0, &[index::REANIMATE, grist()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, kit::P0);
        let grist = hand_to_graveyard(&mut engine, kit::P0, grist());
        let before = identity(&engine, grist);

        reanimate(&mut engine, grist, Some(pay));

        assert!(kit::is_on_battlefield(&engine, grist));
        assert_eq!(kit::controller_of(&engine, grist), kit::P0);
        assert_eq!(kit::owner_of(&engine, grist), kit::P0);
        if pay {
            assert_eq!(kit::face_of(&engine, grist), 1);
            assert_eq!(kit::loyalty(&engine, grist), 3);
            assert_ne!(
                identity(&engine, grist),
                before,
                "exile and return is a new object"
            );
            assert!(types(&engine, grist).contains(TypeSet::PLANESWALKER));
            let colors = kit::colors_of(&engine, grist);
            assert!(colors.contains(Color::Black) && colors.contains(Color::Green));
        } else {
            assert_eq!(kit::face_of(&engine, grist), 0);
            assert!(types(&engine, grist).contains(TypeSet::CREATURE));
        }
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            u64::from(!pay),
            "declining the optional green payment leaves it floating"
        );
    }
}

#[test]
fn another_creature_actually_cast_from_the_graveyard_triggers_after_the_stack() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                grist(),
                forest(),
                forest(),
                forest(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[index::YAWGMOTH_S_WILL, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let grist = on_battlefield(&engine, kit::P0, grist()).unwrap();
    let bears = hand_to_graveyard(&mut engine, kit::P0, index::GRIZZLY_BEARS);
    tap_all_mana(&mut engine, kit::P0);
    cast_with_floating(&mut engine, kit::P0, index::YAWGMOTH_S_WILL);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kit::face_of(&engine, grist), 0);

    engine
        .apply(kit::P0, PlayerAction::CastSpell { card: bears })
        .expect("Yawgmoth's Will really permits the graveyard cast");
    assert_eq!(kit::zone_of(&engine, bears), Zone::Stack);
    assert!(
        engine
            .state()
            .object(bears)
            .is_some_and(|o| o.cast_history.was_from_graveyard()),
        "the cast records its graveyard origin"
    );
    reanimate_payment_only(&mut engine);
    assert!(kit::is_on_battlefield(&engine, bears));
    assert!(
        engine
            .state()
            .object(bears)
            .is_some_and(|o| o.cast_history.was_from_graveyard()),
        "the stack-to-battlefield move retains its actual caster and origin"
    );
    assert_eq!(kit::face_of(&engine, grist), 1);
    assert_eq!(kit::loyalty(&engine, grist), 3);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[track_caller]
fn reanimate_payment_only(engine: &mut Engine<RegistryLookup>) {
    kit::settle(engine, kit::P0, |_, pending| match pending {
        Pending::YesNo {
            prompt: YesNoPrompt::PayMana { cost },
            ..
        } => {
            assert_eq!(*cost, baylee_core::mana!("{G}"));
            Some(PlayerAction::YesNo(true))
        }
        _ => None,
    });
}

#[test]
fn the_transform_may_be_declined_without_a_forest_even_when_green_was_not_spent() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grist(), swamp()])
        .hand(0, &[index::REANIMATE, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let grist = on_battlefield(&engine, kit::P0, grist()).unwrap();
    let bears = hand_to_graveyard(&mut engine, kit::P0, index::GRIZZLY_BEARS);
    reanimate(&mut engine, bears, None);

    assert!(kit::is_on_battlefield(&engine, bears));
    assert_eq!(kit::face_of(&engine, grist), 0);
}

#[test]
fn plus_one_mills_exactly_two_and_marks_the_new_token_only_for_a_black_card() {
    for (top, expected) in [(forest(), 0), (index::DARK_RITUAL, 1)] {
        let (mut engine, grist) = back_at(3);
        let other = on_battlefield(&engine, kit::P0, index::GRAY_OGRE).unwrap();
        let stacked = kit::stack_library(&mut engine, kit::P0, &[top, forest(), mountain()]);
        engine.refresh_offer();

        loyalty_ability(&mut engine, grist, 0);
        kit::settle(&mut engine, kit::P0, |_, _| None);

        let tokens = tokens_of(&engine, kit::P0);
        assert_eq!(tokens.len(), 1);
        let token = tokens[0];
        assert_eq!(pt(&engine, token), (1, 1));
        assert!(kit::has_subtype(&engine, token, creature::INSECT));
        let colors = kit::colors_of(&engine, token);
        assert!(colors.contains(Color::Black) && colors.contains(Color::Green));
        assert_eq!(
            counters_on(&engine, token, CounterKind::Deathtouch),
            expected
        );
        assert_eq!(
            keywords(&engine, token).contains(KeywordSet::DEATHTOUCH),
            expected == 1
        );
        assert_eq!(counters_on(&engine, grist, CounterKind::Deathtouch), 0);
        assert_eq!(counters_on(&engine, other, CounterKind::Deathtouch), 0);
        for milled in &stacked[..2] {
            assert_eq!(kit::zone_of(&engine, *milled), Zone::Graveyard);
        }
        assert_eq!(kit::zone_of(&engine, stacked[2]), Zone::Library);
        assert_eq!(kit::loyalty(&engine, grist), 4);
    }
}

#[test]
fn minus_two_targets_and_destroys_only_an_artifact_or_enchantment() {
    let (mut engine, grist) = back_at(3);
    let tome = on_battlefield(&engine, kit::P1, index::JAYEMDAE_TOME).unwrap();
    let anthem = on_battlefield(&engine, kit::P1, index::GLORIOUS_ANTHEM).unwrap();
    let creature = on_battlefield(&engine, kit::P1, index::GRAY_OGRE).unwrap();

    loyalty_ability(&mut engine, grist, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the −2 asks for its target: {:?}", engine.pending());
    };
    assert_eq!(options, vec![tome, anthem]);
    aim_at(&mut engine, kit::P0, anthem);
    kit::settle(&mut engine, kit::P0, |_, _| None);

    assert_eq!(kit::zone_of(&engine, anthem), Zone::Graveyard);
    assert!(kit::is_on_battlefield(&engine, tome));
    assert!(kit::is_on_battlefield(&engine, creature));
    assert_eq!(kit::loyalty(&engine, grist), 1);
}

#[test]
fn minus_six_copies_each_own_creature_card_with_only_the_printed_exceptions() {
    let (mut engine, grist) = back_at(7);
    let angel = hand_to_graveyard(&mut engine, kit::P0, serra_angel());
    let bears = hand_to_graveyard(&mut engine, kit::P0, index::GRIZZLY_BEARS);
    let bolt = hand_to_graveyard(&mut engine, kit::P0, lightning_bolt());
    let theirs = on_battlefield(&engine, kit::P1, index::GRAY_OGRE).unwrap();
    engine
        .dev_state_mut(kit::P0)
        .unwrap()
        .move_object(
            theirs,
            ZoneLocation::Graveyard(kit::P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    loyalty_ability(&mut engine, grist, 2);
    kit::settle(&mut engine, kit::P0, |_, _| None);

    let tokens = tokens_of(&engine, kit::P0);
    assert_eq!(tokens.len(), 2, "only our two creature cards are copied");
    for token in &tokens {
        assert_eq!(pt(&engine, *token), (1, 1));
        assert!(kit::has_subtype(&engine, *token, creature::INSECT));
        assert!(!kit::has_subtype(&engine, *token, creature::ANGEL));
        assert!(!kit::has_subtype(&engine, *token, creature::BEAR));
        let colors = kit::colors_of(&engine, *token);
        assert!(colors.contains(Color::Black) && colors.contains(Color::Green));
        assert!(!colors.contains(Color::White));
        assert_eq!(kit::controller_of(&engine, *token), kit::P0);
    }
    let angel_copy = tokens
        .iter()
        .copied()
        .find(|token| keywords(&engine, *token).contains(KeywordSet::FLYING))
        .expect("the copied Serra Angel keeps flying");
    assert!(keywords(&engine, angel_copy).contains(KeywordSet::VIGILANCE));
    for original in [angel, bears, bolt, theirs] {
        assert_eq!(kit::zone_of(&engine, original), Zone::Graveyard);
    }
    assert_eq!(kit::loyalty(&engine, grist), 1);
    assert!(kit::is_on_battlefield(&engine, grist));
}

#[test]
fn a_grist_that_dies_to_its_minus_six_is_itself_a_creature_card_to_copy() {
    let (mut engine, grist) = back_at(6);
    loyalty_ability(&mut engine, grist, 2);
    kit::settle(&mut engine, kit::P0, |_, _| None);

    assert_eq!(kit::zone_of(&engine, grist), Zone::Graveyard);
    let tokens = tokens_of(&engine, kit::P0);
    assert_eq!(tokens.len(), 1, "the front face is a creature card");
    let copy = tokens[0];
    assert_eq!(pt(&engine, copy), (1, 1));
    assert!(kit::has_subtype(&engine, copy, creature::INSECT));
    assert!(keywords(&engine, copy).contains(KeywordSet::DEATHTOUCH));
    assert!(types(&engine, copy).contains(TypeSet::CREATURE));
    assert!(!types(&engine, copy).contains(TypeSet::PLANESWALKER));
    assert_eq!(kit::offered(&engine, copy), Vec::<u32>::new());
}
