//! `cards/creatures/mv_2/balduvian_hydra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn balduvian_hydra() -> CardIndex {
    card_index("29a12f75-d66d-457f-85c4-6b9450adc0d8")
}

/// Balduvian Hydra (29a12f75-d66d-457f-85c4-6b9450adc0d8) — {X}{R}{R} 0/1 Creature — Hydra:
/// - "Balduvian Hydra enters with X +1/+0 counters on it."
/// - "Remove a +1/+0 counter from Balduvian Hydra: Prevent the next 1 damage
///   that would be dealt to Balduvian Hydra this turn."
/// - "{R}{R}{R}: Put a +1/+0 counter on Balduvian Hydra. Activate only during
///   your upkeep."
#[test]
fn balduvian_hydra_enters_with_x_plus_one_plus_zero_counters_prevents_damage_and_grows_on_upkeep() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[balduvian_hydra(), spikefield_hazard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Tap 4 Mountains -> X = 2 ({2}{R}{R})
    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, balduvian_hydra());
    if let Pending::ChooseNumber { .. } = engine.pending() {
        engine
            .apply(P0, PlayerAction::ChooseNumber(2))
            .expect("choose X = 2");
    }
    pass_until(&mut engine, stack_is_empty);

    let hydra = on_battlefield(&engine, P0, balduvian_hydra()).expect("Hydra entered");
    assert_eq!(
        counters_on(
            &engine,
            hydra,
            CounterKind::Plus {
                power: 1,
                toughness: 0
            }
        ),
        2
    );
    assert_eq!(pt(&engine, hydra), (2, 1));

    // "{R}{R}{R}: Put a +1/+0 counter on Balduvian Hydra. Activate only during your upkeep."
    // Not legal in FirstMain.
    assert!(
        !priority_offer(&engine).abilities.contains(&(hydra, 1)),
        "upkeep counter ability cannot be activated in main phase"
    );

    // Remove one +1/+0 counter to prevent the next 1 damage dealt to Hydra this turn.
    activate(&mut engine, P0, balduvian_hydra(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, hydra), (1, 1), "one +1/+0 counter removed");

    // Advance to P1's main phase, then to P0's upkeep: activate {R}{R}{R} to put a +1/+0 counter.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, balduvian_hydra(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(
            &engine,
            hydra,
            CounterKind::Plus {
                power: 1,
                toughness: 0
            }
        ),
        2,
        "upkeep ability added a +1/+0 counter"
    );
    assert_eq!(pt(&engine, hydra), (2, 1));

    // In main phase, play a Mountain and ping Hydra for 1 damage after removing a +1/+0 counter to prevent it.
    walk_to_own_main(&mut engine, P0);
    play_land(&mut engine, P0, mountain());
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, balduvian_hydra(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, hydra), (1, 1));

    cast_front_face(&mut engine, P0, spikefield_hazard());
    aim_at(&mut engine, P0, hydra);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, balduvian_hydra()).is_some(),
        "1 damage was prevented so the 1/1 Hydra survived"
    );
    assert_eq!(engine.state().object(hydra).unwrap().damage, 0);
}

/// "Prevent the next 1 damage that would be dealt to it **this turn**": a
/// shield bought on P0's first turn is gone by P0's next one. The Hydra,
/// cast with X = 1, spends its only counter on a shield and is a 0/1; two
/// turns later Spikefield Hazard's 1 damage is not prevented, it dies, and
/// the Hazard exiles it instead.
#[test]
fn balduvian_hydra_s_shield_ends_with_the_turn_it_was_made() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[balduvian_hydra(), spikefield_hazard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Three Mountains -> X = 1 ({1}{R}{R}).
    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, balduvian_hydra());
    if let Pending::ChooseNumber { .. } = engine.pending() {
        engine
            .apply(P0, PlayerAction::ChooseNumber(1))
            .expect("choose X = 1");
    }
    pass_until(&mut engine, stack_is_empty);
    let hydra = on_battlefield(&engine, P0, balduvian_hydra()).expect("Hydra entered");
    assert_eq!(pt(&engine, hydra), (1, 1), "one +1/+0 counter");

    activate(&mut engine, P0, balduvian_hydra(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, hydra),
        (0, 1),
        "its counter paid for the shield"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    tap_all_mana_but(&mut engine, P0, None);
    cast_front_face(&mut engine, P0, spikefield_hazard());
    aim_at(&mut engine, P0, hydra);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, balduvian_hydra()).is_none(),
        "the turn-one shield had ended, so the 1 damage killed the 0/1"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .iter()
            .any(|&id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == balduvian_hydra()))),
        "and Spikefield Hazard exiled it instead"
    );
}
