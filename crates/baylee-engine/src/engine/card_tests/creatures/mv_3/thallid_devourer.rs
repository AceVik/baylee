//! `cards/creatures/mv_3/thallid_devourer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn thallid_devourer() -> CardIndex {
    card_index("b7326fd9-7648-4db3-b506-61a7de8a4094")
}

fn board() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thallid_devourer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let id = on_battlefield(&engine, P0, thallid_devourer()).expect("the Devourer is out");
    (engine, id)
}

fn add_spores(engine: &mut Engine<RegistryLookup>, id: ObjectId, n: u16) {
    crate::replacement::put_counters(&mut engine.state, id, counters::SPORE, n);
    engine.refresh_offer();
}

/// The Saproling tokens P0 controls.
fn saprolings(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == P0)
        })
        .collect()
}

/// Activates ability `index`, answering whatever its cost asks with the
/// first option, and lets it resolve.
#[track_caller]
fn activate_and_resolve(engine: &mut Engine<RegistryLookup>, index: u32) {
    activate(engine, P0, thallid_devourer(), index);
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = answer_one(engine).expect("an answerable cost question");
        engine.apply(player, action).expect("legal");
    }
    pass_until(engine, stack_is_empty);
}

/// Thallid Devourer (b7326fd9-7648-4db3-b506-61a7de8a4094) — "At the
/// beginning of your upkeep, put a spore counter on this creature."
#[test]
fn thallid_devourer_gains_a_spore_counter_each_of_your_upkeeps() {
    let (mut engine, id) = board();
    assert_eq!(counters_on(&engine, id, counters::SPORE), 1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, id, counters::SPORE),
        1,
        "not on their upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, id, counters::SPORE), 2);
}

/// "Remove three spore counters from this creature: Create a 1/1 green
/// Saproling creature token."
#[test]
fn thallid_devourer_makes_a_saproling_for_three_spore_counters() {
    let (mut engine, id) = board();
    add_spores(&mut engine, id, 5);
    assert_eq!(counters_on(&engine, id, counters::SPORE), 6);
    assert!(saprolings(&engine).is_empty());

    activate_and_resolve(&mut engine, 1);
    assert_eq!(counters_on(&engine, id, counters::SPORE), 3);
    let tokens = saprolings(&engine);
    assert_eq!(tokens.len(), 1, "one Saproling");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));

    activate_and_resolve(&mut engine, 1);
    assert_eq!(counters_on(&engine, id, counters::SPORE), 0);
    assert_eq!(saprolings(&engine).len(), 2);
}

#[test]
fn thallid_devourer_needs_three_counters_for_a_saproling() {
    let (mut engine, id) = board();
    add_spores(&mut engine, id, 1);
    assert_eq!(counters_on(&engine, id, counters::SPORE), 2);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: 1
                }
            )
            .is_err(),
        "two counters cannot pay for three"
    );
    assert!(saprolings(&engine).is_empty());
}

/// "Sacrifice a Saproling: This creature gets +1/+2 until end of turn."
/// Each sacrifice stacks; it wears off at the end of the turn.
#[test]
fn thallid_devourer_sacrifices_saprolings_for_plus_one_plus_two() {
    let (mut engine, id) = board();
    add_spores(&mut engine, id, 5);
    activate_and_resolve(&mut engine, 1);
    activate_and_resolve(&mut engine, 1);
    assert_eq!(saprolings(&engine).len(), 2);
    assert_eq!(pt(&engine, id), (2, 2));

    activate_and_resolve(&mut engine, 2);
    assert_eq!(saprolings(&engine).len(), 1, "one Saproling was sacrificed");
    assert_eq!(pt(&engine, id), (3, 4), "+1/+2");

    activate_and_resolve(&mut engine, 2);
    assert!(saprolings(&engine).is_empty());
    assert_eq!(pt(&engine, id), (4, 6), "+1/+2 again, stacking");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, id), (2, 2), "until end of turn only");
}

/// Only a Saproling can be sacrificed: with none, the ability is not offered.
#[test]
fn thallid_devourer_cannot_sacrifice_without_a_saproling() {
    let (mut engine, id) = board();
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: 2
                }
            )
            .is_err(),
        "no Saproling to sacrifice"
    );
    assert_eq!(pt(&engine, id), (2, 2));
}
