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
