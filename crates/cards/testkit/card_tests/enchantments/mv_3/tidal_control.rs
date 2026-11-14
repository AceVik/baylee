//! `cards/1996/src/all/tidal_control.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tidal_control() -> CardIndex {
    card_index("855e200e-5375-4aac-b1f7-5113162e7e14")
}

fn walk_to_pay_tax(engine: &mut Engine<RegistryLookup>, player: PlayerId) -> u16 {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player: p,
                prompt: YesNoPrompt::PayTax { .. },
                ..
            } if *p == player
        )
    });
    let Pending::YesNo {
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("stopped on PayTax prompt");
    };
    mana
}

fn pay_upkeep_with_islands(engine: &mut Engine<RegistryLookup>, player: PlayerId, expected: u16) {
    assert_eq!(walk_to_pay_tax(engine, player), expected);
    engine.apply(player, PlayerAction::YesNo(true)).unwrap();
    if matches!(engine.pending(), Pending::Priority { .. }) {
        let islands = all_on_battlefield(engine, player, island());
        tap_mana_where(engine, player, |id| islands.contains(&id));
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    pass_until(engine, stack_is_empty);
}

fn activate_tidal_counter(
    engine: &mut Engine<RegistryLookup>,
    activator: PlayerId,
    tc: ObjectId,
    ability_index: u32,
    target: ObjectId,
) {
    engine
        .apply(
            activator,
            PlayerAction::ActivateAbility {
                source: tc,
                ability_index,
            },
        )
        .expect("tidal control ability activates");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected spell targets, got {:?}", engine.pending());
    };
    assert!(options.contains(&target), "target spell must be offered");
    engine
        .apply(
            activator,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
}

/// Cumulative upkeep {2}: puts age counter 1 and asks {2}; after paying, the
/// second upkeep puts age counter 2, asks {4}, and sacrifices when declined.
#[test]
fn tidal_control_cumulative_upkeep_escalates_and_sacrifices_on_decline() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 4])
        .hand(0, &[tidal_control()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands[..3].contains(&id));
    cast_with_floating(&mut engine, P0, tidal_control());
    pass_until(&mut engine, stack_is_empty);
    let tc = on_battlefield(&engine, P0, tidal_control()).expect("Tidal Control resolves");

    pay_upkeep_with_islands(&mut engine, P0, 2);
    assert_eq!(counters_on(&engine, tc, counters::AGE), 1);
    assert!(on_battlefield(&engine, P0, tidal_control()).is_some());

    assert_eq!(walk_to_pay_tax(&mut engine, P0), 4);
    assert_eq!(counters_on(&engine, tc, counters::AGE), 2);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, tidal_control()).is_none());
    assert!(in_graveyard(&engine, P0, tidal_control()).is_some());
}

/// Controller pays {2} to counter a green spell, non-red/non-green spells
/// cannot be targeted, and an opponent may pay 2 life to counter a red spell.
#[test]
fn tidal_control_counters_red_or_green_and_rejects_other_colors() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), mountain()],
        )
        .battlefield(1, &[forest(), forest(), island()])
        .hand(0, &[tidal_control(), lightning_bolt()])
        .hand(1, &[grizzly_bears(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands[..3].contains(&id));
    cast_with_floating(&mut engine, P0, tidal_control());
    pass_until(&mut engine, stack_is_empty);
    let tc = on_battlefield(&engine, P0, tidal_control()).expect("Tidal Control on board");

    // P1 casts green Grizzly Bears; P0 floats {2} and activates ability 1 ({2}).
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, grizzly_bears());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let bears_spell = on_stack(&engine, grizzly_bears()).expect("Bears on stack");
    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands.contains(&id));
    activate_tidal_counter(&mut engine, P0, tc, 1, bears_spell);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());

    // P1 casts colorless Sol Ring: neither {2} nor Pay 2 life may target it.
    cast_from_hand(&mut engine, P1, sol_ring());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected P0 priority");
    };
    assert!(!legal.abilities.iter().any(|(s, _)| *s == tc));
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: tc,
                    ability_index: 2,
                },
            )
            .is_err()
    );
    pass_until(&mut engine, stack_is_empty);

    // Next P0 main: pay {2} upkeep, then P0 casts red Bolt; P1 pays 2 life!
    pay_upkeep_with_islands(&mut engine, P0, 2);
    reach_main_phase(&mut engine, P0);

    let mtns = all_on_battlefield(&engine, P0, mountain());
    tap_mana_where(&mut engine, P0, |id| mtns.contains(&id));
    cast_with_floating(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    let bolt_spell = on_stack(&engine, lightning_bolt()).expect("Bolt on stack");
    activate_tidal_counter(&mut engine, P1, tc, 2, bolt_spell);
    assert_eq!(life_of(&engine, P1), 18);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, lightning_bolt()).is_some());
    assert_eq!(life_of(&engine, P1), 18, "countered Bolt dealt 0 damage");
}

/// Opponent can also activate with {2} mana, and Pay 2 life obeys CR 119.4
/// (illegal at 1 life, legal at 2 life).
#[test]
fn tidal_control_opponent_pays_mana_and_checks_life_boundary() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
                wall_of_swords(),
            ],
        )
        .battlefield(1, &[island(), island()])
        .hand(
            0,
            &[
                tidal_control(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands.contains(&id));
    cast_with_floating(&mut engine, P0, tidal_control());
    pass_until(&mut engine, stack_is_empty);
    let tc = on_battlefield(&engine, P0, tidal_control()).expect("Tidal Control on board");
    let wall = on_battlefield(&engine, P0, wall_of_swords()).expect("Wall on board");

    // Bolt 1: P1 activates ability 1 paying {2} mana.
    let mtns = all_on_battlefield(&engine, P0, mountain());
    tap_mana_where(&mut engine, P0, |id| mtns.contains(&id));
    cast_with_floating(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![wall],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    tap_all_mana(&mut engine, P1);
    let bolt1 = on_stack(&engine, lightning_bolt()).expect("Bolt 1 on stack");
    activate_tidal_counter(&mut engine, P1, tc, 1, bolt1);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    // Set P1 life to 1 -> Pay 2 life (ability 2) is illegal on Bolt 2.
    engine.dev_state_mut(P1).unwrap().players[1].life = 1;
    cast_with_floating(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![wall],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected P1 priority");
    };
    assert!(!legal.abilities.iter().any(|(s, idx)| *s == tc && *idx == 2));
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ActivateAbility {
                    source: tc,
                    ability_index: 2,
                },
            )
            .is_err()
    );
    pass_until(&mut engine, stack_is_empty);

    // Set P1 life to 2 -> CR 119.4 allows paying 2 life on Bolt 3.
    engine.dev_state_mut(P1).unwrap().players[1].life = 2;
    cast_with_floating(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![wall],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    let bolt3 = on_stack(&engine, lightning_bolt()).expect("Bolt 3 on stack");
    activate_tidal_counter(&mut engine, P1, tc, 2, bolt3);
    assert_eq!(life_of(&engine, P1), 0);
}
