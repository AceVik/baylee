//! Venarian Gold — {X}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, tap enchanted creature and put X sleep counters on it.
//! Oracle: Enchanted creature doesn't untap during its controller's untap step if it has a sleep counter on it.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, remove a sleep counter from that creature.
//! Set: LEG #83 — Legends | Scryfall ID: 11fb92c0-bb1e-463a-a6b6-887a5d0cb873 | Oracle ID: 5d7acb89-1778-409a-9e91-aaeadd13ca27

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VENARIAN_GOLD,
    oracle_id = "5d7acb89-1778-409a-9e91-aaeadd13ca27",
    scryfall_id = "11fb92c0-bb1e-463a-a6b6-887a5d0cb873",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Venarian Gold",
        mana_cost = mana!("{X}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::ETB,
            &[
                Effect::TapAll {
                    filter: &Filter::AttachedToBySource
                },
                Effect::ChangeAttachedCounters {
                    kind: counters::SLEEP,
                    amount: Amount::X,
                    remove: false
                },
            ]
        ),
        static_ability!(
            Filter::And(&[
                Filter::AttachedToBySource,
                Filter::HasCounter(counters::SLEEP)
            ]),
            Modifier::DoesNotUntap
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached
            },
            &[Effect::ChangeAttachedCounters {
                kind: counters::SLEEP,
                amount: Amount::Fixed(1),
                remove: true
            }]
        ),
    ],
);
