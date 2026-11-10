//! Dread Wight — {3}{B}{B} — Creature — Zombie
//! Oracle: At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains "{4}: Remove a paralyzation counter from this creature."
//! Set: ME4 #79 — Masters Edition IV | Scryfall ID: bc719b18-949b-4d84-a7bf-f2e0c2eb5d25 | Oracle ID: f9078d79-3250-44e8-aba1-ab52e0ba4425
// IMPLEMENTED — per creature: an untap lock while it keeps a paralyzation counter, and the {4} grant with no stated duration (CR 611.2a).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_OR_BLOCKED_BY_SOURCE: Filter =
    Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]);

card!(
    index = index::DREAD_WIGHT,
    oracle_id = "f9078d79-3250-44e8-aba1-ab52e0ba4425",
    scryfall_id = "bc719b18-949b-4d84-a7bf-f2e0c2eb5d25",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dread Wight",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatEnd,
            whose: PlayerRel::EachPlayer,
        },
        &[
            Effect::AddCounterFilter {
                filter: &BLOCKING_OR_BLOCKED_BY_SOURCE,
                kind: counters::PARALYZATION,
                amount: Amount::Fixed(1),
            },
            Effect::TapAll {
                filter: &BLOCKING_OR_BLOCKED_BY_SOURCE,
            },
            // "Each of those creatures doesn't untap … for as long as it has
            // a paralyzation counter on it": a rule, not a characteristic, so
            // a filter would stay live (CR 611.2c). Bound to each creature
            // in turn, as the event object, for as long as its own counter
            // lasts.
            Effect::ForEachMatching {
                filter: &BLOCKING_OR_BLOCKED_BY_SOURCE,
                effects: &[Effect::ModifyEvent {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileCounterRemains(counters::PARALYZATION),
                }],
            },
            // "Each of those creatures gains …": an ability is a
            // characteristic, so the set is locked as this resolves
            // (CR 611.2c), and no duration is stated (CR 611.2a).
            Effect::continuous(
                &BLOCKING_OR_BLOCKED_BY_SOURCE,
                Modifier::GrantActivated {
                    cost: cost!("{4}"),
                    effects: &[Effect::RemoveCounterSelf {
                        kind: counters::PARALYZATION,
                        n: 1,
                    }],
                    mana_ability: false,
                },
                Duration::Indefinitely,
            ),
        ],
    )],
);
