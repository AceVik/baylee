//! Dread Wight — {3}{B}{B} — Creature — Zombie
//! Oracle: At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains "{4}: Remove a paralyzation counter from this creature."
//! Set: ME4 #79 — Masters Edition IV | Scryfall ID: bc719b18-949b-4d84-a7bf-f2e0c2eb5d25 | Oracle ID: f9078d79-3250-44e8-aba1-ab52e0ba4425
// PARTIAL: paralyzation-counter untap lock and granted ability apply as static abilities while Dread Wight is on the battlefield rather than a counter-duration continuous effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_OR_BLOCKED_BY_SOURCE: Filter =
    Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]);
const CREATURE_WITH_PARALYZATION: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasCounter(counters::PARALYZATION)]);

// NOT SUPPORTED: Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains "{4}: Remove a paralyzation counter from this creature." (modeled as static abilities on Dread Wight).
card!(
    index = index::DREAD_WIGHT,
    oracle_id = "f9078d79-3250-44e8-aba1-ab52e0ba4425",
    scryfall_id = "bc719b18-949b-4d84-a7bf-f2e0c2eb5d25",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "paralyzation-counter untap lock and granted ability apply as static abilities while Dread Wight is on the battlefield rather than a counter-duration continuous effect"
    ),
    faces = &[face!(
        name = "Dread Wight",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
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
            ],
        ),
        static_ability!(CREATURE_WITH_PARALYZATION, Modifier::DoesNotUntap),
        static_ability!(
            CREATURE_WITH_PARALYZATION,
            Modifier::GrantActivated {
                cost: cost!("{4}"),
                effects: &[Effect::RemoveCounterSelf {
                    kind: counters::PARALYZATION,
                    n: 1,
                }],
                mana_ability: false,
            }
        ),
    ],
);
