//! Isolated journal fixtures for event predicates without a registered spell
//! that creates the unusual combat. These use the actual registered cards;
//! they do not add synthetic card definitions or claim full combat legality.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::{DamageTarget, GameEvent};

fn bears(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.controller == P0 && o.card.is_some_and(|c| c.index == index::GRIZZLY_BEARS)
            })
        })
        .collect()
}

fn landing_events(e: &mut Engine<RegistryLookup>, declared: &[ObjectId]) -> usize {
    let landing = source(e, P0, index::LEGION_S_LANDING);
    let from = e.state.journal.last_seq();
    for &object in declared {
        // Mirror the real declaration's historical ledger as well as its
        // journal; an incorrect turn-wide count must therefore fail the test.
        let version = e.state.object(object).unwrap().version;
        if !e.state.per_turn.attacked.contains(&(object, version, P0)) {
            e.state.per_turn.attacked.push((object, version, P0));
        }
        e.state.journal.record(GameEvent::BecameAttacker {
            object,
            defending: Defender::Player(P1),
        });
    }
    crate::trigger::collect(&e.state, &e.lookup, from)
        .iter()
        .filter(|t| t.source == landing)
        .count()
}

#[test]
fn landing_separate_declarations_do_not_accumulate_toward_three() {
    let mut e = prepared(
        &[],
        &[
            index::LEGION_S_LANDING,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let creatures = bears(&e);
    assert_eq!(landing_events(&mut e, &creatures[..1]), 0);
    assert_eq!(landing_events(&mut e, &creatures[1..]), 0);
    assert_eq!(
        e.state.per_turn.attacked.len(),
        3,
        "all three attacked during this turn"
    );
    // The same three declared together in another extra combat do qualify.
    assert_eq!(landing_events(&mut e, &creatures), 1);
}

#[test]
fn landing_creature_put_attacking_does_not_count_as_declared() {
    let mut e = prepared(
        &[],
        &[
            index::LEGION_S_LANDING,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let creatures = bears(&e);
    // Represents the board/combat result of an effect putting a creature onto
    // the battlefield attacking. Such an effect does not emit BecameAttacker.
    e.state
        .combat
        .declare_attackers([crate::combat::AttackerInfo {
            creature: creatures[0],
            defending: Defender::Player(P1),
            blocked: false,
            band: None,
        }]);
    e.state
        .combat
        .declare_attackers(
            creatures[1..]
                .iter()
                .map(|&creature| crate::combat::AttackerInfo {
                    creature,
                    defending: Defender::Player(P1),
                    blocked: false,
                    band: None,
                }),
        );
    assert_eq!(
        e.state.combat.attackers().len(),
        3,
        "three are attacking, but only two were declared"
    );
    assert_eq!(landing_events(&mut e, &creatures[1..]), 0);
    assert_eq!(e.state.per_turn.attacked.len(), 2);
}

#[test]
fn storm_groups_from_first_matching_damage_source_not_first_any_source() {
    for first_kind in 0..3 {
        let mut e = prepared(
            &[],
            &[
                index::STORM_THE_VAULT,
                index::SOL_RING,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
            ],
            &[index::GRIZZLY_BEARS],
        );
        let vault = source(&e, P0, index::STORM_THE_VAULT);
        let creatures = bears(&e);
        // Source-less, noncreature, and other-controller damage each precede
        // two qualifying creature records to the same recipient in one batch.
        let first = match first_kind {
            0 => None,
            1 => Some(source(&e, P0, index::SOL_RING)),
            _ => Some(source(&e, P1, index::GRIZZLY_BEARS)),
        };
        let from = e.state.journal.last_seq();
        for damage_source in [first, Some(creatures[0]), Some(creatures[1])] {
            e.state.journal.record(GameEvent::DamageDealt {
                source: damage_source,
                target: DamageTarget::Player(P1),
                amount: 2,
                is_combat: true,
            });
        }
        let triggers = crate::trigger::collect(&e.state, &e.lookup, from);
        let matching: Vec<_> = triggers.iter().filter(|t| t.source == vault).collect();
        assert_eq!(
            matching.len(),
            1,
            "one qualifying grouped event; preceding kind {first_kind}"
        );
        assert_eq!(
            matching[0].event_object,
            Some(creatures[0]),
            "first MATCHING source defines the grouped trigger"
        );
        assert_eq!(matching[0].event_player, Some(P1));
    }
}
