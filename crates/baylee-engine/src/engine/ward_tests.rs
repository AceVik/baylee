//! Ward through real casts, responses and the ordinary choice API.
use super::testkit::*;
use super::*;
use crate::choice::YesNoPrompt;
use baylee_core::ids::CardIndex;

fn card(name: &str) -> CardIndex {
    baylee_cards::generated::ALL
        .iter()
        .find(|(_, c)| c.name() == name)
        .unwrap_or_else(|| panic!("missing fixture card: {name}"))
        .1
        .index
}
fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}
fn aim(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    let Pending::ChooseTargets {
        player,
        ref options,
        ..
    } = *engine.pending()
    else {
        panic!("expected targets: {:?}", engine.pending());
    };
    assert!(options.contains(&target));
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
}
fn wait_for_payment(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { .. }),
        "ward did not ask: {:?}",
        engine.pending()
    );
}
fn targeted_fleshgorger(life: i32) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(401, card("Plains"))
        .battlefield(0, &[card("Phyrexian Fleshgorger")])
        .battlefield(1, &[card("Plains")])
        .hand(1, &[card("Swords to Plowshares")])
        .life(1, life)
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    let flesh = on_battlefield(&engine, seat(0), card("Phyrexian Fleshgorger")).unwrap();
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, flesh);
    (engine, flesh)
}

#[test]
fn fleshgorger_offers_life_payment_to_the_spell_controller_and_payment_works() {
    let (mut engine, flesh) = targeted_fleshgorger(20);
    wait_for_payment(&mut engine);
    assert!(matches!(engine.pending(), Pending::YesNo {
        player, prompt: YesNoPrompt::PayLife { amount: 7 }, ..
    } if *player == seat(1)));
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 13);
    assert_eq!(engine.state().object(flesh).unwrap().zone, Zone::Exile);
}

#[test]
fn declining_life_ward_counters_the_spell_without_spending_life() {
    let (mut engine, flesh) = targeted_fleshgorger(20);
    wait_for_payment(&mut engine);
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(
        engine.state().object(flesh).unwrap().zone,
        Zone::Battlefield
    );
    assert!(in_graveyard(&engine, seat(1), card("Swords to Plowshares")).is_some());
}

#[test]
fn insufficient_life_cannot_pay_ward_and_no_impossible_payment_is_offered() {
    let (mut engine, flesh) = targeted_fleshgorger(6);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 6);
    assert_eq!(
        engine.state().object(flesh).unwrap().zone,
        Zone::Battlefield
    );
    assert!(in_graveyard(&engine, seat(1), card("Swords to Plowshares")).is_some());
}

#[test]
fn ward_does_not_tax_a_teammates_spell() {
    let mut engine = Duel::table(402, card("Plains"), 3)
        .team(0, 1)
        .team(1, 1)
        .team(2, 2)
        .battlefield(0, &[card("Twining Twins")])
        .battlefield(1, &[card("Plains")])
        .hand(1, &[card("Swords to Plowshares")])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    let twins = on_battlefield(&engine, seat(0), card("Twining Twins")).unwrap();
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, twins);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(twins).unwrap().zone, Zone::Exile);
}

#[test]
fn retargeting_an_opponents_spell_onto_ward_taxes_its_controller_not_the_redirector() {
    let mut engine = Duel::new(403, card("Plains"))
        .battlefield(
            0,
            &[
                card("Twining Twins"),
                card("Ondu Cleric"),
                card("Mountain"),
                card("Mountain"),
                card("Mountain"),
            ],
        )
        .hand(0, &[card("Deflecting Swat")])
        .battlefield(1, &[card("Plains")])
        .hand(1, &[card("Swords to Plowshares")])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    let twins = on_battlefield(&engine, seat(0), card("Twining Twins")).unwrap();
    let cleric = on_battlefield(&engine, seat(0), card("Ondu Cleric")).unwrap();
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, cleric);
    let swords = on_stack(&engine, card("Swords to Plowshares")).unwrap();
    engine.apply(seat(1), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, seat(0), card("Deflecting Swat"));
    aim(&mut engine, swords);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, twins);
    wait_for_payment(&mut engine);
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayTax { mana: 1 }, .. } if *player == seat(1))
    );
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(twins).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(cleric).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn hall_animation_grants_ward_and_stacking_animations_creates_independent_instances() {
    let mut board = vec![card("Hall of Storm Giants")];
    board.extend([card("Island"); 12]);
    let mut engine = Duel::new(404, card("Plains"))
        .battlefield(0, &board)
        .battlefield(
            1,
            &[
                card("Plains"),
                card("Plains"),
                card("Plains"),
                card("Plains"),
            ],
        )
        .hand(1, &[card("Swords to Plowshares")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    let hall = on_battlefield(&engine, seat(0), card("Hall of Storm Giants")).unwrap();
    tap_mana_except(&mut engine, seat(0), hall);
    for _ in 0..2 {
        engine
            .apply(
                seat(0),
                PlayerAction::ActivateAbility {
                    source: hall,
                    ability_index: 1,
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, hall);
    // Two identical triggers are automatically ordered by the engine.
    wait_for_payment(&mut engine);
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayTax { mana: 3 },
            ..
        }
    ));
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    wait_for_payment(&mut engine);
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(hall).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

#[test]
fn activated_abilities_trigger_ward_and_keep_their_paid_activation_cost_when_countered() {
    let mut engine = Duel::new(405, card("Plains"))
        .battlefield(0, &[card("Twining Twins")])
        .battlefield(1, &[card("Auriok Bladewarden")])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    let twins = on_battlefield(&engine, seat(0), card("Twining Twins")).unwrap();
    let warden = on_battlefield(&engine, seat(1), card("Auriok Bladewarden")).unwrap();
    engine
        .apply(
            seat(1),
            PlayerAction::ActivateAbility {
                source: warden,
                ability_index: 0,
            },
        )
        .unwrap();
    aim(&mut engine, twins);
    wait_for_payment(&mut engine);
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, twins), (4, 4));
    assert!(
        engine
            .state()
            .object(warden)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn a_spell_copy_triggers_its_own_ward_without_being_cast() {
    let mut engine = Duel::new(406, card("Plains"))
        .battlefield(0, &[card("Twining Twins")])
        .battlefield(
            1,
            &[
                card("Storm of Saruman"),
                card("Plains"),
                card("Plains"),
                card("Plains"),
            ],
        )
        .hand(1, &[card("Ornithopter"), card("Swords to Plowshares")])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), card("Ornithopter"));
    pass_until(&mut engine, stack_is_empty);
    let twins = on_battlefield(&engine, seat(0), card("Twining Twins")).unwrap();
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, twins);
    wait_for_payment(&mut engine);
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, twins);
    wait_for_payment(&mut engine);
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayTax { mana: 1 }, .. } if *player == seat(1))
    );
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.spells_cast[1],
        2,
        "a copy is not a cast"
    );
    assert_eq!(
        engine.state().object(twins).unwrap().zone,
        Zone::Exile,
        "the original still resolves"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "only the first ward was paid"
    );
}

#[test]
fn life_ward_reads_power_at_resolution_and_keeps_last_known_power_across_a_blink() {
    for blink in [false, true] {
        let mut engine = Duel::new(407, card("Plains"))
            .battlefield(
                0,
                &[
                    card("Phyrexian Fleshgorger"),
                    card("Forest"),
                    card("Plains"),
                ],
            )
            .hand(0, &[card("Giant Growth"), card("Ephemerate")])
            .battlefield(1, &[card("Plains")])
            .hand(1, &[card("Swords to Plowshares")])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, seat(1));
        let flesh = on_battlefield(&engine, seat(0), card("Phyrexian Fleshgorger")).unwrap();
        cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
        aim(&mut engine, flesh);
        engine.apply(seat(1), PlayerAction::PassPriority).unwrap();
        cast_from_hand(&mut engine, seat(0), card("Giant Growth"));
        aim(&mut engine, flesh);
        pass_until(&mut engine, |e| on_stack(e, card("Giant Growth")).is_none());
        assert_eq!(pt(&engine, flesh), (10, 8));
        if blink {
            pass_until(
                &mut engine,
                |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat(0)),
            );
            cast_from_hand(&mut engine, seat(0), card("Ephemerate"));
            aim(&mut engine, flesh);
            pass_until(&mut engine, |e| on_stack(e, card("Ephemerate")).is_none());
            let returned = on_battlefield(&engine, seat(0), card("Phyrexian Fleshgorger")).unwrap();
            assert_eq!(pt(&engine, returned), (7, 5));
        }
        wait_for_payment(&mut engine);
        assert!(
            matches!(engine.pending(), Pending::YesNo {
            player, prompt: YesNoPrompt::PayLife { amount: 10 }, ..
        } if *player == seat(1)),
            "blink={blink}: {:?}",
            engine.pending()
        );
        engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[1].life, 10);
    }
}

#[test]
fn hall_loses_its_granted_ward_at_end_of_turn() {
    let mut board = vec![card("Hall of Storm Giants")];
    board.extend([card("Island"); 6]);
    let mut engine = Duel::new(409, card("Plains"))
        .battlefield(0, &board)
        .battlefield(1, &[card("Mountain"); 3])
        .hand(1, &[card("Stone Rain")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    let hall = on_battlefield(&engine, seat(0), card("Hall of Storm Giants")).unwrap();
    tap_mana_except(&mut engine, seat(0), hall);
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateAbility {
                source: hall,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), card("Stone Rain"));
    aim(&mut engine, hall);
    // A surviving ward grant would stop here to ask for payment.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(hall).unwrap().zone, Zone::Graveyard);
}

#[test]
fn prototype_is_castable_for_three_and_ward_uses_its_actual_power() {
    let mut engine = Duel::new(421, card("Swamp"))
        .battlefield(0, &[card("Swamp"); 3])
        .hand(0, &[card("Phyrexian Fleshgorger")])
        .battlefield(1, &[card("Plains")])
        .hand(1, &[card("Swords to Plowshares")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), card("Phyrexian Fleshgorger"));
    let flesh = engine.state().zones.list(ZoneLocation::Stack)[0];
    let assert_prototype = |engine: &Engine<RegistryLookup>| {
        let obj = engine.state().object(flesh).unwrap();
        assert!(obj.prototyped);
        assert_eq!(obj.characteristics().power, Some(3));
        assert_eq!(obj.characteristics().toughness, Some(3));
        assert_eq!(
            obj.characteristics().mana_cost,
            baylee_core::mana::ManaCost::try_parse("{1}{B}{B}").unwrap()
        );
        assert_eq!(
            obj.characteristics().colors,
            baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black])
        );
    };
    assert_prototype(&engine);
    pass_until(&mut engine, stack_is_empty);
    assert_prototype(&engine);
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, flesh);
    wait_for_payment(&mut engine);
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayLife { amount: 3 },
            ..
        }
    ));
    engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let exiled = engine.state().object(flesh).unwrap();
    assert!(!exiled.prototyped);
    assert_eq!(exiled.characteristics().power, Some(7));
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn thorins_story_counts_permanents_once_and_keeps_the_designation() {
    for (artifacts, earns) in [
        (vec![card("Sol Ring")], false),
        (vec![card("Sol Ring"), card("Arcane Signet")], true),
    ] {
        let mut battlefield = vec![card("Thorin Oakenshield")];
        battlefield.extend(artifacts);
        let mut engine = Duel::new(422, card("Plains"))
            .battlefield(0, &battlefield)
            .battlefield(1, &[card("Plains"); 2])
            .hand(1, &[card("Swords to Plowshares")])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, seat(1));
        assert_eq!(engine.state().players[0].enduring_story, earns);
        assert!(!engine.state().players[1].enduring_story);
        let thorin = on_battlefield(&engine, seat(0), card("Thorin Oakenshield")).unwrap();
        cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
        aim(&mut engine, thorin);
        if earns {
            wait_for_payment(&mut engine);
            engine.apply(seat(1), PlayerAction::YesNo(true)).unwrap();
        }
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().object(thorin).unwrap().zone, Zone::Exile);
        assert_eq!(engine.state().players[0].enduring_story, earns);
    }
}

fn disguised_branch() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(423, card("Plains"))
        .battlefield(0, &[card("Plains"); 6])
        .hand(0, &[card("Branch of Vitu-Ghazi")])
        .battlefield(1, &[card("Plains"); 3])
        .hand(1, &[card("Swords to Plowshares")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    cast_from_hand(&mut engine, seat(0), card("Branch of Vitu-Ghazi"));
    pass_until(&mut engine, stack_is_empty);
    let branch = on_battlefield(&engine, seat(0), card("Branch of Vitu-Ghazi")).unwrap();
    let o = engine.state().object(branch).unwrap();
    assert!(o.status.contains(crate::object::Status::FACE_DOWN));
    assert_eq!(o.characteristics().power, Some(2));
    assert_eq!(
        o.characteristics().types,
        baylee_core::types::TypeSet::CREATURE
    );
    assert_eq!(
        o.characteristics().mana_cost,
        baylee_core::mana::ManaCost::ZERO
    );
    (engine, branch)
}

#[test]
fn disguised_land_has_ward_two_and_not_its_printed_mana_ability() {
    let (mut engine, branch) = disguised_branch();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    assert!(!legal.mana_abilities.contains(&branch));
    assert!(!legal.abilities.contains(&(branch, 0)));
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), card("Swords to Plowshares"));
    aim(&mut engine, branch);
    let ward = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .copied()
        .unwrap();
    let ward = engine.state().object(ward).unwrap();
    let loc = ward.ability.expect("ward must be a stack ability");
    assert_eq!(loc.card, None, "ward must not reveal the face-down card");
    assert_eq!(ward.printed_face(), None);
    wait_for_payment(&mut engine);
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { source: None, .. }
    ));
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(branch).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn disguise_turns_face_up_as_a_special_action_and_mana_lasts_only_this_turn() {
    let (mut engine, branch) = disguised_branch();
    let version = engine.state().object(branch).unwrap().version;
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateAbility {
                source: branch,
                ability_index: crate::choice::TURN_FACE_UP,
            },
        )
        .unwrap();
    let o = engine.state().object(branch).unwrap();
    assert!(!o.status.contains(crate::object::Status::FACE_DOWN));
    assert_eq!(o.version, version, "turning face up is not a zone change");
    assert_eq!(o.characteristics().types, baylee_core::types::TypeSet::LAND);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "only the face-up trigger goes on the stack"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue),
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Blue),
        2
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Blue),
        2
    );
    reach_their_main_phase(&mut engine, seat(1));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Blue),
        0
    );
}

#[test]
fn every_printed_mana_ward_card_in_the_pool_counters_a_declined_hostile_spell() {
    for name in [
        "Twining Twins",
        "Tyrranax Rex",
        "Roaming Throne",
        "Storm of Saruman",
    ] {
        let mut engine = Duel::new(425, card("Plains"))
            .battlefield(0, &[card(name)])
            .battlefield(1, &[card("Plains"); 10])
            .hand(
                1,
                &[card(if name == "Storm of Saruman" {
                    "Disenchant"
                } else {
                    "Swords to Plowshares"
                })],
            )
            .start();
        keep_mulligans(&mut engine);
        if let Pending::ChooseSubtype {
            player,
            ref options,
        } = *engine.pending()
        {
            engine
                .apply(player, PlayerAction::ChooseSubtype(options[0]))
                .unwrap();
        }
        reach_their_main_phase(&mut engine, seat(1));
        let target = on_battlefield(&engine, seat(0), card(name)).unwrap();
        cast_from_hand(
            &mut engine,
            seat(1),
            card(if name == "Storm of Saruman" {
                "Disenchant"
            } else {
                "Swords to Plowshares"
            }),
        );
        aim(&mut engine, target);
        wait_for_payment(&mut engine);
        engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().object(target).unwrap().zone,
            Zone::Battlefield,
            "{name}"
        );
        assert!(
            in_graveyard(
                &engine,
                seat(1),
                card(if name == "Storm of Saruman" {
                    "Disenchant"
                } else {
                    "Swords to Plowshares"
                })
            )
            .is_some(),
            "{name}"
        );
    }
}
