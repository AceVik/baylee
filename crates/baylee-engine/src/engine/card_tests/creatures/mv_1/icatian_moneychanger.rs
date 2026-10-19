//! `cards/creatures/mv_1/icatian_moneychanger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn credits(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state()
        .object(id)
        .expect("the Moneychanger is out")
        .counters
        .get(baylee_cards_dsl::counters::CREDIT)
}

/// The Moneychanger cast and resolved in player 0's first main phase.
fn cast() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[card_index("9b81aa32-e0f7-493f-92b8-85c2c46aef91")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    fill_pools(&mut e);
    cast_with_floating(
        &mut e,
        P0,
        card_index("9b81aa32-e0f7-493f-92b8-85c2c46aef91"),
    );
    pass_until(&mut e, stack_is_empty);
    let mc = on_battlefield(&e, P0, card_index("9b81aa32-e0f7-493f-92b8-85c2c46aef91"))
        .expect("it resolved");
    (e, mc)
}

fn offers_sacrifice(e: &Engine<RegistryLookup>, mc: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    legal.abilities.contains(&(mc, 2))
}

/// Icatian Moneychanger (9b81aa32-e0f7-493f-92b8-85c2c46aef91) — "This
/// creature enters with three credit counters on it. When this creature
/// enters, it deals 3 damage to you. At the beginning of your upkeep, put a
/// credit counter on this creature. Sacrifice this creature: You gain 1
/// life for each credit counter on this creature. Activate only during your
/// upkeep."
#[test]
fn icatian_moneychanger_enters_with_three_credit_counters_and_hurts_you_for_three() {
    let (e, mc) = cast();
    assert_eq!(credits(&e, mc), 3);
    assert_eq!(e.state().players[0].life, 17, "3 damage to its controller");
    assert_eq!(e.state().players[1].life, 20);
    assert_eq!(pt(&e, mc), (0, 2));
}

#[test]
fn icatian_moneychanger_gains_a_counter_on_your_upkeep_only() {
    let (mut e, mc) = cast();
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(credits(&e, mc), 3, "the opponent's upkeep adds nothing");

    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(credits(&e, mc), 4, "your upkeep adds one");
}

#[test]
fn icatian_moneychanger_sacrificed_in_your_upkeep_gains_life_per_counter() {
    let (mut e, mc) = cast();
    assert!(
        !offers_sacrifice(&e, mc),
        "not in the main phase: upkeep only"
    );

    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(credits(&e, mc), 4);
    let life = e.state().players[0].life;
    assert!(offers_sacrifice(&e, mc), "in your upkeep it is offered");

    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: mc,
            ability_index: 2,
        },
    )
    .expect("the sacrifice needs no mana");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, life + 4, "one life per counter");
    assert!(
        on_battlefield(&e, P0, card_index("9b81aa32-e0f7-493f-92b8-85c2c46aef91")).is_none(),
        "it was sacrificed"
    );
    assert!(in_graveyard(&e, P0, card_index("9b81aa32-e0f7-493f-92b8-85c2c46aef91")).is_some());
}
